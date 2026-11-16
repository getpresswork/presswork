use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use presswork_api::Role;
use presswork_api::live::{self, ClientMessage, PROTOCOL, ServerMessage};
use presswork_domain::{Action, allowed};
use presswork_sync::SyncError;
use tokio::sync::mpsc;
use tokio::time::timeout;
use uuid::Uuid;

use crate::hub::{HubError, Outbox, Outgoing, Room};
use crate::routes::AppState;
use crate::{documents, links, sessions};

const HELLO_TIME: Duration = Duration::from_secs(10);
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

static CONNECTIONS: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn upgrade(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| session(socket, state))
}

struct Connection {
    id: u64,
    user: Uuid,
    outbox: Outbox,
    subscriptions: HashMap<Uuid, bool>,
}

async fn session(mut socket: WebSocket, state: AppState) {
    let Some(user) = hello(&mut socket, &state).await else {
        return;
    };
    let (outbox, mut inbox) = mpsc::unbounded_channel();
    let mut connection = Connection {
        id: CONNECTIONS.fetch_add(1, Ordering::Relaxed),
        user,
        outbox,
        subscriptions: HashMap::new(),
    };
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Binary(bytes))) => {
                    if let Err(answer) = connection.handle(&state, &bytes).await
                        && send(&mut socket, &answer).await.is_err()
                    {
                        break;
                    }
                }
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            outgoing = inbox.recv() => match outgoing {
                Some(Outgoing::Message(message)) => {
                    if send(&mut socket, &message).await.is_err() {
                        break;
                    }
                }
                Some(Outgoing::Close) | None => break,
            },
        }
    }
    for document in connection.subscriptions.keys() {
        state.hub.leave(*document, connection.id).await;
    }
}

async fn hello(socket: &mut WebSocket, state: &AppState) -> Option<Uuid> {
    let hello = match timeout(HELLO_TIME, socket.recv()).await {
        Ok(Some(Ok(Message::Binary(bytes)))) => live::decode::<ClientMessage>(&bytes).ok(),
        _ => None,
    };
    let Some(ClientMessage::Hello { protocol, token }) = hello else {
        refuse(socket, invalid("The first message must be Hello.")).await;
        return None;
    };
    if protocol != PROTOCOL {
        refuse(
            socket,
            error(
                "unsupported_protocol",
                "The server uses a different version of the live protocol.",
            ),
        )
        .await;
        return None;
    }
    match sessions::find(&state.database, &token).await {
        Ok(Some(signed)) => {
            send(
                socket,
                &ServerMessage::Welcome {
                    user: signed.user_id,
                },
            )
            .await
            .ok()?;
            Some(signed.user_id)
        }
        Ok(None) => {
            refuse(
                socket,
                error("unauthenticated", "Sign in to use this function."),
            )
            .await;
            None
        }
        Err(problem) => {
            refuse(socket, failed(&problem)).await;
            None
        }
    }
}

impl Connection {
    async fn handle(&mut self, state: &AppState, bytes: &[u8]) -> Result<(), ServerMessage> {
        match live::decode::<ClientMessage>(bytes) {
            Ok(ClientMessage::Subscribe { document, version }) => {
                self.subscribe(state, document, &version).await
            }
            Ok(ClientMessage::Updates { document, id, data }) => {
                self.publish(state, document, id, data).await
            }
            Ok(ClientMessage::Hello { .. }) => {
                Err(invalid("The connection has a session already."))
            }
            Err(_) => Err(invalid("The message is not valid.")),
        }
    }

    async fn subscribe(
        &mut self,
        state: &AppState,
        document: Uuid,
        version: &[u8],
    ) -> Result<(), ServerMessage> {
        let role = role(state, self.user, document).await?;
        let room = room(state, document).await?;
        room.subscribe(self.id, self.outbox.clone(), version)
            .await
            .map_err(hub_error)?;
        self.subscriptions
            .insert(document, allowed(role, Action::EditDocuments));
        Ok(())
    }

    async fn publish(
        &self,
        state: &AppState,
        document: Uuid,
        id: u64,
        data: Vec<u8>,
    ) -> Result<(), ServerMessage> {
        match self.subscriptions.get(&document) {
            None => {
                return Err(invalid(
                    "Subscribe to the document before you send changes.",
                ));
            }
            Some(false) => return Err(forbidden()),
            Some(true) => {}
        }
        let room = room(state, document).await?;
        let pool = state.database.clone();
        let author = self.user;
        room.publish(self.id, id, data, |data| async move {
            documents::append_update(&pool, document, author, &data).await
        })
        .await
        .map_err(hub_error)
    }
}

async fn role(state: &AppState, user: Uuid, document: Uuid) -> Result<Role, ServerMessage> {
    let organization = documents::document_organization(&state.database, document)
        .await
        .map_err(|problem| failed(&problem))?
        .ok_or_else(not_found)?;
    let role = links::role(&state.database, organization, user)
        .await
        .map_err(|problem| failed(&problem))?
        .ok_or_else(not_found)?;
    if allowed(role, Action::ReadProjects) {
        Ok(role)
    } else {
        Err(forbidden())
    }
}

async fn room(state: &AppState, document: Uuid) -> Result<Arc<Room>, ServerMessage> {
    let pool = state.database.clone();
    state
        .hub
        .room(document, || async move {
            documents::updates(&pool, document).await
        })
        .await
        .map_err(hub_error)
}

fn hub_error(problem: HubError) -> ServerMessage {
    match problem {
        HubError::Sync(SyncError::Version) => invalid("The version is not valid."),
        HubError::Sync(SyncError::Update(_)) => invalid("The change is not valid."),
        HubError::Sync(problem) => failed(&problem),
        HubError::Store(problem) => failed(&problem),
    }
}

fn failed(problem: &dyn fmt::Display) -> ServerMessage {
    tracing::error!(error = %problem, "A live request failed");
    error("internal", "The server cannot complete the request.")
}

fn invalid(message: &str) -> ServerMessage {
    error("invalid_request", message)
}

fn not_found() -> ServerMessage {
    error("not_found", "The item does not exist.")
}

fn forbidden() -> ServerMessage {
    error(
        "forbidden",
        "You do not have the permission for this function.",
    )
}

fn error(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), axum::Error> {
    let bytes = live::encode(message).map_err(axum::Error::new)?;
    socket.send(Message::Binary(bytes.into())).await
}

async fn refuse(socket: &mut WebSocket, message: ServerMessage) {
    let _ = send(socket, &message).await;
    let _ = socket.send(Message::Close(None)).await;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use presswork_api::Role;
    use presswork_api::live::{ClientMessage, ServerMessage};
    use presswork_client::{ClientError, Live};
    use presswork_sync::TextDocument;
    use presswork_text::{ChangeSet, Edit, Position};
    use sqlx::PgPool;
    use tokio::net::TcpListener;
    use uuid::Uuid;

    use crate::accounts::create_user;
    use crate::documents::{self, create_document};
    use crate::links::create_organization;
    use crate::routes::{AppState, router};
    use crate::sessions;
    use crate::testing::test_database;
    use crate::work::create_project;

    const ANSWER_TIME: Duration = Duration::from_secs(10);

    struct Press {
        server: String,
        ann: String,
        bob: String,
        eve: String,
        document: Uuid,
    }

    async fn press(pool: &PgPool) -> Press {
        let organization = create_organization(pool, "Field Press").await.unwrap().id;
        let mut tokens = Vec::new();
        let mut users = Vec::new();
        for (email, member) in [
            ("ann@example.com", true),
            ("bob@example.com", true),
            ("eve@example.com", false),
        ] {
            let user = create_user(pool, email, "Someone", "correct horse battery")
                .await
                .unwrap();
            if member {
                sqlx::query(
                    "INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, $3)",
                )
                .bind(organization)
                .bind(user.id)
                .bind(Role::Member.as_str())
                .execute(pool)
                .await
                .unwrap();
            }
            let (token, _) = sessions::start(pool, user.id, "Laptop").await.unwrap();
            tokens.push(token);
            users.push(user.id);
        }
        let project = create_project(pool, organization, "Field Notes")
            .await
            .unwrap();
        let document = create_document(pool, project.id, users[0], "main.typ")
            .await
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = router(AppState::new(pool.clone()));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let [ann, bob, eve] = <[String; 3]>::try_from(tokens).unwrap();
        Press {
            server: format!("http://{address}"),
            ann,
            bob,
            eve,
            document: document.id,
        }
    }

    struct Person {
        live: Live,
        text: TextDocument,
        document: Uuid,
    }

    impl Person {
        async fn connect(press: &Press, token: &str) -> Live {
            Live::connect(&press.server, token).await.unwrap()
        }

        async fn join(press: &Press, token: &str) -> Self {
            let mut live = Self::connect(press, token).await;
            let mut text = TextDocument::new();
            live.send(&ClientMessage::Subscribe {
                document: press.document,
                version: text.version(),
            })
            .await
            .unwrap();
            let ServerMessage::Subscribed { data, .. } = next(&mut live).await else {
                panic!("the server did not send the document");
            };
            text.import(&data).unwrap();
            Self {
                live,
                text,
                document: press.document,
            }
        }

        async fn write(&mut self, at: usize, words: &str, id: u64) {
            let length = self.text.text().len();
            let data = self
                .text
                .apply(&ChangeSet::new(length, [Edit::insert(Position::new(at), words)]).unwrap())
                .unwrap();
            self.live
                .send(&ClientMessage::Updates {
                    document: self.document,
                    id,
                    data,
                })
                .await
                .unwrap();
        }

        async fn settle(&mut self, acks: usize, updates: usize) {
            let (mut acks, mut updates) = (acks, updates);
            while acks > 0 || updates > 0 {
                match next(&mut self.live).await {
                    ServerMessage::Ack { .. } => {
                        acks = acks.checked_sub(1).expect("an extra acknowledgement");
                    }
                    ServerMessage::Updates { data, .. } => {
                        self.text.import(&data).unwrap();
                        updates = updates.checked_sub(1).expect("an extra update");
                    }
                    other => panic!("an unexpected message: {other:?}"),
                }
            }
        }
    }

    async fn next(live: &mut Live) -> ServerMessage {
        tokio::time::timeout(ANSWER_TIME, live.receive())
            .await
            .expect("the server answered in time")
            .unwrap()
            .expect("the connection is open")
    }

    fn code(message: &ServerMessage) -> &str {
        match message {
            ServerMessage::Error { code, .. } => code,
            other => panic!("an error was expected, not {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_live_connection_needs_a_valid_session() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = press(&database.pool).await;

        let refused = Live::connect(&press.server, "pws_not_a_session").await;

        assert!(
            matches!(&refused, Err(ClientError::Refused { code, .. }) if code == "unauthenticated"),
            "{refused:?}"
        );
    }

    #[tokio::test]
    async fn two_persons_edit_one_document_live() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = press(&database.pool).await;
        let mut ann = Person::join(&press, &press.ann).await;
        let mut bob = Person::join(&press, &press.bob).await;

        ann.write(0, "The press stopped.", 1).await;
        ann.settle(1, 0).await;
        bob.settle(0, 1).await;
        assert_eq!(bob.text.text(), "The press stopped.");

        ann.write(0, "Late: ", 2).await;
        bob.write(17, " at noon", 1).await;
        ann.settle(1, 1).await;
        bob.settle(1, 1).await;

        assert_eq!(ann.text.text(), "Late: The press stopped at noon.");
        assert_eq!(bob.text.text(), ann.text.text());
        let stored = documents::updates(&database.pool, press.document)
            .await
            .unwrap();
        assert_eq!(stored.len(), 3);
        assert_eq!(
            TextDocument::load_all(&stored).unwrap().text(),
            ann.text.text()
        );
        let late = Person::join(&press, &press.bob).await;
        assert_eq!(late.text.text(), ann.text.text());
    }

    #[tokio::test]
    async fn an_outsider_cannot_open_a_document() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = press(&database.pool).await;
        let mut eve = Person::connect(&press, &press.eve).await;

        eve.send(&ClientMessage::Subscribe {
            document: press.document,
            version: TextDocument::new().version(),
        })
        .await
        .unwrap();

        assert_eq!(code(&next(&mut eve).await), "not_found");
    }

    #[tokio::test]
    async fn changes_need_a_subscription_and_a_valid_update() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = press(&database.pool).await;
        let mut ann = Person::connect(&press, &press.ann).await;

        ann.send(&ClientMessage::Updates {
            document: press.document,
            id: 1,
            data: b"garbage".to_vec(),
        })
        .await
        .unwrap();
        assert_eq!(code(&next(&mut ann).await), "invalid_request");

        let mut ann = Person::join(&press, &press.ann).await;
        ann.live
            .send(&ClientMessage::Updates {
                document: press.document,
                id: 2,
                data: b"garbage".to_vec(),
            })
            .await
            .unwrap();
        assert_eq!(code(&next(&mut ann.live).await), "invalid_request");
        assert_eq!(
            documents::updates(&database.pool, press.document)
                .await
                .unwrap(),
            Vec::<Vec<u8>>::new()
        );
    }
}
