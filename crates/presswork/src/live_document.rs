use std::time::Duration;

use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use iced::Subscription;
use iced::futures::Stream;
use presswork_api::Document;
use presswork_api::live::{ClientMessage, ServerMessage};
use presswork_client::Live;
use presswork_sync::{SyncError, TextDocument};
use presswork_text::ChangeSet;
use uuid::Uuid;

use crate::server_dialog::Connection;

const RETRY: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Connecting,
    Live,
    Offline,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Ready(mpsc::UnboundedSender<ClientMessage>),
    Connected,
    Received(ServerMessage),
    Disconnected,
}

pub(crate) struct LiveDocument {
    connection: Connection,
    document: Document,
    text: TextDocument,
    state: State,
    outbox: Option<mpsc::UnboundedSender<ClientMessage>>,
    next_id: u64,
}

impl LiveDocument {
    pub(crate) fn new(connection: Connection, document: Document) -> Self {
        Self {
            connection,
            document,
            text: TextDocument::new(),
            state: State::Connecting,
            outbox: None,
            next_id: 1,
        }
    }

    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn state(&self) -> State {
        self.state
    }

    pub(crate) fn local(&mut self, changes: &ChangeSet) -> Result<(), SyncError> {
        let data = self.text.apply(changes)?;
        if self.state == State::Live {
            self.send_updates(data);
        }
        Ok(())
    }

    pub(crate) fn event(&mut self, event: Event) -> Result<Option<ChangeSet>, SyncError> {
        let own = self.document.id;
        match event {
            Event::Ready(outbox) => self.outbox = Some(outbox),
            Event::Connected => {
                self.state = State::Connecting;
                self.send(ClientMessage::Subscribe {
                    document: own,
                    version: self.text.version(),
                });
            }
            Event::Received(ServerMessage::Subscribed {
                document,
                version,
                data,
            }) if document == own => {
                let changes = self.text.import(&data)?;
                let missing = self.text.updates_since(&version)?;
                self.state = State::Live;
                self.send_updates(missing);
                return Ok(Some(changes));
            }
            Event::Received(ServerMessage::Updates { document, data }) if document == own => {
                return Ok(Some(self.text.import(&data)?));
            }
            Event::Received(ServerMessage::Error { code, message }) => {
                tracing::warn!(%code, %message, "The server refused a live request");
            }
            Event::Received(_) => {}
            Event::Disconnected => self.state = State::Offline,
        }
        Ok(None)
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with(
            Key {
                server: self.connection.api.server().to_owned(),
                token: self.connection.token.clone(),
                document: self.document.id,
            },
            connect,
        )
    }

    fn send_updates(&mut self, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.send(ClientMessage::Updates {
            document: self.document.id,
            id,
            data,
        });
    }

    fn send(&self, message: ClientMessage) {
        if let Some(outbox) = &self.outbox {
            let _ = outbox.unbounded_send(message);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    server: String,
    token: String,
    document: Uuid,
}

fn connect(key: &Key) -> impl Stream<Item = Event> + use<> {
    let key = key.clone();
    iced::stream::channel(64, async move |mut output: mpsc::Sender<Event>| {
        let (outbox, mut inbox) = mpsc::unbounded();
        if output.send(Event::Ready(outbox)).await.is_err() {
            return;
        }
        loop {
            match Live::connect(&key.server, &key.token).await {
                Ok(mut live) => {
                    while inbox.try_recv().is_ok() {}
                    if output.send(Event::Connected).await.is_err() {
                        return;
                    }
                    loop {
                        tokio::select! {
                            received = live.receive() => match received {
                                Ok(Some(message)) => {
                                    if output.send(Event::Received(message)).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(None) | Err(_) => break,
                            },
                            outgoing = inbox.next() => match outgoing {
                                Some(message) => {
                                    if live.send(&message).await.is_err() {
                                        break;
                                    }
                                }
                                None => return,
                            },
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "The live connection did not start"),
            }
            if output.send(Event::Disconnected).await.is_err() {
                return;
            }
            tokio::time::sleep(RETRY).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc;
    use presswork_api::Document;
    use presswork_api::live::{ClientMessage, ServerMessage};
    use presswork_client::Api;
    use presswork_sync::TextDocument;
    use presswork_text::{Buffer, ChangeSet, Edit, Position};
    use uuid::Uuid;

    use super::*;
    use crate::server_dialog::Connection;

    const MAIN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0004);

    fn session() -> (LiveDocument, mpsc::UnboundedReceiver<ClientMessage>) {
        let connection = Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
        };
        let document = Document {
            id: MAIN,
            project_id: Uuid::nil(),
            path: "main.typ".to_owned(),
        };
        let mut live = LiveDocument::new(connection, document);
        let (sender, receiver) = mpsc::unbounded();
        live.event(Event::Ready(sender)).unwrap();
        (live, receiver)
    }

    fn server_text(text: &str) -> TextDocument {
        let mut server = TextDocument::new();
        server
            .apply(&ChangeSet::new(0, [Edit::insert(Position::new(0), text)]).unwrap())
            .unwrap();
        server
    }

    fn sent(receiver: &mut mpsc::UnboundedReceiver<ClientMessage>) -> Vec<ClientMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    fn subscribed(live: &mut LiveDocument, server: &TextDocument) -> Option<ChangeSet> {
        let data = server
            .updates_since(&TextDocument::new().version())
            .unwrap();
        live.event(Event::Received(ServerMessage::Subscribed {
            document: MAIN,
            version: server.version(),
            data,
        }))
        .unwrap()
    }

    fn applied(text: &str, changes: &ChangeSet) -> String {
        let mut buffer = Buffer::from(text);
        buffer.apply(changes).unwrap();
        buffer.to_string()
    }

    fn insert(length: usize, at: usize, words: &str) -> ChangeSet {
        ChangeSet::new(length, [Edit::insert(Position::new(at), words)]).unwrap()
    }

    fn updates(messages: &[ClientMessage]) -> Vec<Vec<u8>> {
        messages
            .iter()
            .filter_map(|message| match message {
                ClientMessage::Updates { data, .. } => Some(data.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_connection_subscribes_with_the_version_of_the_document() {
        let (mut live, mut receiver) = session();
        assert_eq!(live.state(), State::Connecting);

        live.event(Event::Connected).unwrap();

        assert!(matches!(
            sent(&mut receiver).as_slice(),
            [ClientMessage::Subscribe { document: MAIN, .. }]
        ));
    }

    #[test]
    fn the_subscription_brings_the_text_of_the_server() {
        let (mut live, _receiver) = session();
        live.event(Event::Connected).unwrap();

        let changes = subscribed(&mut live, &server_text("Proof")).unwrap();

        assert_eq!(applied("", &changes), "Proof");
        assert_eq!(live.state(), State::Live);
    }

    #[test]
    fn local_changes_go_to_the_server_while_live() {
        let (mut live, mut receiver) = session();
        let mut server = server_text("Proof");
        live.event(Event::Connected).unwrap();
        subscribed(&mut live, &server);
        sent(&mut receiver);

        live.local(&insert(5, 5, "s")).unwrap();

        let data = updates(&sent(&mut receiver));
        assert_eq!(data.len(), 1);
        server.merge(&data[0]).unwrap();
        assert_eq!(server.text(), "Proofs");
    }

    #[test]
    fn remote_updates_give_changes_for_the_editor() {
        let (mut live, _receiver) = session();
        let mut server = server_text("Proof");
        live.event(Event::Connected).unwrap();
        subscribed(&mut live, &server);
        let data = server.apply(&insert(5, 0, "The ")).unwrap();

        let changes = live
            .event(Event::Received(ServerMessage::Updates {
                document: MAIN,
                data,
            }))
            .unwrap()
            .unwrap();

        assert_eq!(applied("Proof", &changes), "The Proof");
    }

    #[test]
    fn changes_while_offline_go_to_the_server_after_the_next_subscription() {
        let (mut live, mut receiver) = session();
        let mut server = server_text("Proof");
        live.event(Event::Connected).unwrap();
        subscribed(&mut live, &server);
        live.event(Event::Disconnected).unwrap();
        sent(&mut receiver);

        live.local(&insert(5, 5, "s")).unwrap();
        assert_eq!(sent(&mut receiver), []);
        assert_eq!(live.state(), State::Offline);
        live.event(Event::Connected).unwrap();
        subscribed(&mut live, &server);

        let data = updates(&sent(&mut receiver));
        assert_eq!(data.len(), 1);
        server.merge(&data[0]).unwrap();
        assert_eq!(server.text(), "Proofs");
        assert_eq!(live.state(), State::Live);
    }
}
