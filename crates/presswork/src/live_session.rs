use std::time::Duration;

use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use iced::Subscription;
use iced::futures::Stream;
use presswork_api::Document;
use presswork_api::live::{ChatMessage, ClientMessage, ServerMessage};
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

struct LiveDocument {
    document: Document,
    text: TextDocument,
    subscribed: bool,
    next_id: u64,
}

#[derive(Debug)]
pub(crate) struct Chat {
    organization: Uuid,
    name: String,
    messages: Vec<ChatMessage>,
}

impl Chat {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    fn add(&mut self, message: ChatMessage) {
        if !self.messages.iter().any(|known| known.id == message.id) {
            self.messages.push(message);
        }
    }
}

pub(crate) struct LiveSession {
    connection: Connection,
    outbox: Option<mpsc::UnboundedSender<ClientMessage>>,
    connected: bool,
    offline: bool,
    document: Option<LiveDocument>,
    chat: Option<Chat>,
}

impl LiveSession {
    pub(crate) fn new(connection: Connection) -> Self {
        Self {
            connection,
            outbox: None,
            connected: false,
            offline: false,
            document: None,
            chat: None,
        }
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    pub(crate) fn document(&self) -> Option<&Document> {
        self.document.as_ref().map(|live| &live.document)
    }

    pub(crate) fn state(&self) -> Option<State> {
        let live = self.document.as_ref()?;
        Some(if self.offline {
            State::Offline
        } else if self.connected && live.subscribed {
            State::Live
        } else {
            State::Connecting
        })
    }

    pub(crate) fn chat(&self) -> Option<&Chat> {
        self.chat.as_ref()
    }

    pub(crate) fn open_document(&mut self, document: Document) {
        self.document = Some(LiveDocument {
            document,
            text: TextDocument::new(),
            subscribed: false,
            next_id: 1,
        });
        if self.connected {
            self.subscribe();
        }
    }

    pub(crate) fn close_document(&mut self) {
        self.document = None;
    }

    pub(crate) fn join_chat(&mut self, organization: Uuid, name: String) {
        if self
            .chat
            .as_ref()
            .is_some_and(|chat| chat.organization == organization)
        {
            return;
        }
        self.chat = Some(Chat {
            organization,
            name,
            messages: Vec::new(),
        });
        if self.connected {
            self.send(ClientMessage::Join { organization });
        }
    }

    pub(crate) fn say(&mut self, text: &str) -> bool {
        let Some(chat) = &self.chat else {
            return false;
        };
        if !self.connected {
            return false;
        }
        let message = ClientMessage::Say {
            organization: chat.organization,
            text: text.to_owned(),
        };
        self.send(message);
        true
    }

    pub(crate) fn local(&mut self, changes: &ChangeSet) -> Result<(), SyncError> {
        let Some(live) = &mut self.document else {
            return Ok(());
        };
        let data = live.text.apply(changes, None)?;
        if self.connected && live.subscribed {
            self.send_updates(data);
        }
        Ok(())
    }

    pub(crate) fn event(&mut self, event: Event) -> Result<Option<ChangeSet>, SyncError> {
        match event {
            Event::Ready(outbox) => self.outbox = Some(outbox),
            Event::Connected => {
                self.connected = true;
                self.offline = false;
                self.subscribe();
                if let Some(chat) = &self.chat {
                    let organization = chat.organization;
                    self.send(ClientMessage::Join { organization });
                }
            }
            Event::Received(message) => return self.receive(message),
            Event::Disconnected => {
                self.connected = false;
                self.offline = true;
                if let Some(live) = &mut self.document {
                    live.subscribed = false;
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with(
            Key {
                server: self.connection.api.server().to_owned(),
                token: self.connection.token.clone(),
            },
            connect,
        )
    }

    fn receive(&mut self, message: ServerMessage) -> Result<Option<ChangeSet>, SyncError> {
        match message {
            ServerMessage::Subscribed {
                document,
                version,
                data,
            } => {
                let Some(live) = self.live_document(document) else {
                    return Ok(None);
                };
                let changes = live.text.import(&data)?.changes;
                let missing = live.text.updates_since(&version)?;
                live.subscribed = true;
                self.send_updates(missing);
                Ok(Some(changes))
            }
            ServerMessage::Updates { document, data } => match self.live_document(document) {
                Some(live) => Ok(Some(live.text.import(&data)?.changes)),
                None => Ok(None),
            },
            ServerMessage::Joined {
                organization,
                messages,
            } => {
                if let Some(chat) = self.chat_of(organization) {
                    let earlier = std::mem::replace(&mut chat.messages, messages);
                    for message in earlier {
                        chat.add(message);
                    }
                }
                Ok(None)
            }
            ServerMessage::Said { message } => {
                if let Some(chat) = self.chat_of(message.organization) {
                    chat.add(message);
                }
                Ok(None)
            }
            ServerMessage::Error { code, message } => {
                tracing::warn!(%code, %message, "The server refused a live request");
                Ok(None)
            }
            ServerMessage::Welcome { .. }
            | ServerMessage::Ack { .. }
            | ServerMessage::WorkChanged { .. } => Ok(None),
        }
    }

    fn live_document(&mut self, document: Uuid) -> Option<&mut LiveDocument> {
        self.document
            .as_mut()
            .filter(|live| live.document.id == document)
    }

    fn chat_of(&mut self, organization: Uuid) -> Option<&mut Chat> {
        self.chat
            .as_mut()
            .filter(|chat| chat.organization == organization)
    }

    fn subscribe(&mut self) {
        let Some(live) = &mut self.document else {
            return;
        };
        live.subscribed = false;
        let message = ClientMessage::Subscribe {
            document: live.document.id,
            version: live.text.version(),
        };
        self.send(message);
    }

    fn send_updates(&mut self, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        let Some(live) = &mut self.document else {
            return;
        };
        let id = live.next_id;
        live.next_id += 1;
        let message = ClientMessage::Updates {
            document: live.document.id,
            id,
            data,
        };
        self.send(message);
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
    use presswork_api::live::{ChatMessage, ClientMessage, ServerMessage};
    use presswork_client::Api;
    use presswork_sync::TextDocument;
    use presswork_text::{Buffer, ChangeSet, Edit, Position};
    use uuid::Uuid;

    use super::*;
    use crate::server_dialog::Connection;

    const MAIN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0004);
    const PRESS: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001);

    fn connection() -> Connection {
        Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
            user_name: "Ann".to_owned(),
        }
    }

    fn document() -> Document {
        Document {
            id: MAIN,
            project_id: Uuid::nil(),
            path: "main.typ".to_owned(),
        }
    }

    fn session() -> (LiveSession, mpsc::UnboundedReceiver<ClientMessage>) {
        let mut session = LiveSession::new(connection());
        let (sender, receiver) = mpsc::unbounded();
        session.event(Event::Ready(sender)).unwrap();
        (session, receiver)
    }

    fn server_text(text: &str) -> TextDocument {
        let mut server = TextDocument::new();
        server
            .apply(
                &ChangeSet::new(0, [Edit::insert(Position::new(0), text)]).unwrap(),
                None,
            )
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

    fn subscribed(session: &mut LiveSession, server: &TextDocument) -> Option<ChangeSet> {
        let data = server
            .updates_since(&TextDocument::new().version())
            .unwrap();
        session
            .event(Event::Received(ServerMessage::Subscribed {
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

    fn chat_message(id: u128, text: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::from_u128(id),
            organization: PRESS,
            author: Uuid::nil(),
            author_name: "Bob".to_owned(),
            text: text.to_owned(),
            sent_at: jiff::Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn a_connection_subscribes_the_document_and_joins_the_chat() {
        let (mut session, mut receiver) = session();
        session.open_document(document());
        session.join_chat(PRESS, "Field Press".to_owned());
        assert_eq!(sent(&mut receiver), []);

        session.event(Event::Connected).unwrap();

        let messages = sent(&mut receiver);
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, ClientMessage::Subscribe { document: MAIN, .. }))
        );
        assert!(messages.contains(&ClientMessage::Join {
            organization: PRESS
        }));
    }

    #[test]
    fn a_document_opened_while_connected_subscribes_at_once() {
        let (mut session, mut receiver) = session();
        session.event(Event::Connected).unwrap();

        session.open_document(document());

        assert!(matches!(
            sent(&mut receiver).as_slice(),
            [ClientMessage::Subscribe { document: MAIN, .. }]
        ));
        assert_eq!(session.state(), Some(State::Connecting));
    }

    #[test]
    fn the_subscription_brings_the_text_of_the_server() {
        let (mut session, _receiver) = session();
        session.open_document(document());
        session.event(Event::Connected).unwrap();

        let changes = subscribed(&mut session, &server_text("Proof")).unwrap();

        assert_eq!(applied("", &changes), "Proof");
        assert_eq!(session.state(), Some(State::Live));
    }

    #[test]
    fn local_changes_go_to_the_server_while_live() {
        let (mut session, mut receiver) = session();
        let mut server = server_text("Proof");
        session.open_document(document());
        session.event(Event::Connected).unwrap();
        subscribed(&mut session, &server);
        sent(&mut receiver);

        session.local(&insert(5, 5, "s")).unwrap();

        let data = updates(&sent(&mut receiver));
        assert_eq!(data.len(), 1);
        server.merge(&data[0]).unwrap();
        assert_eq!(server.text(), "Proofs");
    }

    #[test]
    fn changes_while_offline_go_to_the_server_after_the_next_subscription() {
        let (mut session, mut receiver) = session();
        let mut server = server_text("Proof");
        session.open_document(document());
        session.event(Event::Connected).unwrap();
        subscribed(&mut session, &server);
        session.event(Event::Disconnected).unwrap();
        sent(&mut receiver);

        session.local(&insert(5, 5, "s")).unwrap();
        assert_eq!(sent(&mut receiver), []);
        assert_eq!(session.state(), Some(State::Offline));
        session.event(Event::Connected).unwrap();
        subscribed(&mut session, &server);

        let data = updates(&sent(&mut receiver));
        assert_eq!(data.len(), 1);
        server.merge(&data[0]).unwrap();
        assert_eq!(server.text(), "Proofs");
    }

    #[test]
    fn remote_updates_give_changes_for_the_editor() {
        let (mut session, _receiver) = session();
        let mut server = server_text("Proof");
        session.open_document(document());
        session.event(Event::Connected).unwrap();
        subscribed(&mut session, &server);
        let data = server.apply(&insert(5, 0, "The "), None).unwrap();

        let changes = session
            .event(Event::Received(ServerMessage::Updates {
                document: MAIN,
                data,
            }))
            .unwrap()
            .unwrap();

        assert_eq!(applied("Proof", &changes), "The Proof");
    }

    #[test]
    fn the_chat_keeps_each_message_one_time() {
        let (mut session, _receiver) = session();
        session.join_chat(PRESS, "Field Press".to_owned());
        session.event(Event::Connected).unwrap();

        session
            .event(Event::Received(ServerMessage::Joined {
                organization: PRESS,
                messages: vec![chat_message(1, "Ready?")],
            }))
            .unwrap();
        for message in [chat_message(1, "Ready?"), chat_message(2, "Yes.")] {
            session
                .event(Event::Received(ServerMessage::Said { message }))
                .unwrap();
        }

        let chat = session.chat().unwrap();
        let texts: Vec<&str> = chat
            .messages()
            .iter()
            .map(|message| message.text.as_str())
            .collect();
        assert_eq!(texts, ["Ready?", "Yes."]);
        assert_eq!(chat.name(), "Field Press");
    }

    #[test]
    fn say_sends_the_text_only_while_connected() {
        let (mut session, mut receiver) = session();
        session.join_chat(PRESS, "Field Press".to_owned());

        assert!(!session.say("Too early."));
        session.event(Event::Connected).unwrap();
        sent(&mut receiver);
        assert!(session.say("The proofs are ready."));

        assert_eq!(
            sent(&mut receiver),
            [ClientMessage::Say {
                organization: PRESS,
                text: "The proofs are ready.".to_owned(),
            }]
        );
    }
}
