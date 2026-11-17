use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use presswork_api::live::{ChatMessage, ServerMessage};
use presswork_sync::{SyncError, TextDocument};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::chat::ChatError;
use crate::documents::DocumentError;

#[derive(Debug)]
pub(crate) enum Outgoing {
    Message(ServerMessage),
    Close,
}

pub(crate) type Outbox = mpsc::UnboundedSender<Outgoing>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum HubError {
    #[error(transparent)]
    Sync(#[from] SyncError),
    #[error(transparent)]
    Store(#[from] DocumentError),
}

#[derive(Default)]
pub(crate) struct Hub {
    rooms: Mutex<HashMap<Uuid, Arc<Room>>>,
    chats: Mutex<HashMap<Uuid, HashMap<u64, Outbox>>>,
}

impl Hub {
    pub(crate) async fn join<F, Fut>(
        &self,
        organization: Uuid,
        connection: u64,
        outbox: Outbox,
        recent: F,
    ) -> Result<(), ChatError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<ChatMessage>, ChatError>>,
    {
        let mut chats = self.chats.lock().await;
        let messages = recent().await?;
        let joined = ServerMessage::Joined {
            organization,
            messages,
        };
        if outbox.send(Outgoing::Message(joined)).is_ok() {
            chats
                .entry(organization)
                .or_default()
                .insert(connection, outbox);
        }
        Ok(())
    }

    pub(crate) async fn announce(&self, message: ChatMessage) {
        let organization = message.organization;
        self.tell(organization, &ServerMessage::Said { message })
            .await;
    }

    pub(crate) async fn tell(&self, organization: Uuid, message: &ServerMessage) {
        let chats = self.chats.lock().await;
        if let Some(members) = chats.get(&organization) {
            for outbox in members.values() {
                let _ = outbox.send(Outgoing::Message(message.clone()));
            }
        }
    }

    pub(crate) async fn leave_chat(&self, organization: Uuid, connection: u64) {
        let mut chats = self.chats.lock().await;
        if let Some(members) = chats.get_mut(&organization) {
            members.remove(&connection);
            if members.is_empty() {
                chats.remove(&organization);
            }
        }
    }

    pub(crate) async fn room<F, Fut>(&self, document: Uuid, load: F) -> Result<Arc<Room>, HubError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<Vec<u8>>, DocumentError>>,
    {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get(&document)
            && !room.is_closed().await
        {
            return Ok(Arc::clone(room));
        }
        let text = TextDocument::load_all(&load().await?)?;
        let room = Arc::new(Room::new(document, text));
        rooms.insert(document, Arc::clone(&room));
        Ok(room)
    }

    pub(crate) async fn leave(&self, document: Uuid, connection: u64) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get(&document)
            && room.remove(connection).await
        {
            rooms.remove(&document);
        }
    }
}

impl fmt::Debug for Hub {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Hub").finish_non_exhaustive()
    }
}

pub(crate) struct Room {
    document: Uuid,
    state: Mutex<State>,
}

struct State {
    text: TextDocument,
    subscribers: HashMap<u64, Outbox>,
    closed: bool,
}

impl Room {
    fn new(document: Uuid, text: TextDocument) -> Self {
        Self {
            document,
            state: Mutex::new(State {
                text,
                subscribers: HashMap::new(),
                closed: false,
            }),
        }
    }

    async fn is_closed(&self) -> bool {
        self.state.lock().await.closed
    }

    pub(crate) async fn subscribe(
        &self,
        connection: u64,
        outbox: Outbox,
        version: &[u8],
    ) -> Result<(), HubError> {
        let mut state = self.state.lock().await;
        let message = ServerMessage::Subscribed {
            document: self.document,
            data: state.text.updates_since(version)?,
            version: state.text.version(),
        };
        if outbox.send(Outgoing::Message(message)).is_ok() {
            state.subscribers.insert(connection, outbox);
        }
        Ok(())
    }

    pub(crate) async fn publish<F, Fut>(
        &self,
        connection: u64,
        id: u64,
        data: Vec<u8>,
        store: F,
    ) -> Result<(), HubError>
    where
        F: FnOnce(Vec<u8>) -> Fut,
        Fut: Future<Output = Result<(), DocumentError>>,
    {
        let mut state = self.state.lock().await;
        if !data.is_empty() {
            state.text.merge(&data)?;
            if let Err(error) = store(data.clone()).await {
                for outbox in state.subscribers.values() {
                    let _ = outbox.send(Outgoing::Close);
                }
                state.subscribers.clear();
                state.closed = true;
                return Err(error.into());
            }
        }
        for (subscriber, outbox) in &state.subscribers {
            let message = if *subscriber == connection {
                ServerMessage::Ack {
                    document: self.document,
                    id,
                }
            } else if data.is_empty() {
                continue;
            } else {
                ServerMessage::Updates {
                    document: self.document,
                    data: data.clone(),
                }
            };
            let _ = outbox.send(Outgoing::Message(message));
        }
        Ok(())
    }

    async fn remove(&self, connection: u64) -> bool {
        let mut state = self.state.lock().await;
        state.subscribers.remove(&connection);
        state.subscribers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use presswork_api::live::{ChatMessage, ServerMessage};
    use presswork_sync::TextDocument;
    use presswork_text::{ChangeSet, Edit, Position};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    use super::*;
    use crate::documents::DocumentError;

    const DOCUMENT: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_00d1);

    fn stored(text: &str) -> Vec<Vec<u8>> {
        let mut document = TextDocument::new();
        vec![
            document
                .apply(&ChangeSet::new(0, [Edit::insert(Position::new(0), text)]).unwrap())
                .unwrap(),
        ]
    }

    fn outbox() -> (Outbox, mpsc::UnboundedReceiver<Outgoing>) {
        mpsc::unbounded_channel()
    }

    fn next(inbox: &mut mpsc::UnboundedReceiver<Outgoing>) -> Outgoing {
        inbox.try_recv().expect("a message waits in the outbox")
    }

    async fn room(hub: &Hub, text: &str) -> Arc<Room> {
        let updates = stored(text);
        hub.room(DOCUMENT, || async move { Ok(updates) })
            .await
            .unwrap()
    }

    fn edit_of(data: &[u8], text: &str) -> Vec<u8> {
        let mut document = TextDocument::load(data).unwrap();
        let end = document.text().len();
        document
            .apply(&ChangeSet::new(end, [Edit::insert(Position::new(end), text)]).unwrap())
            .unwrap()
    }

    async fn subscribed(
        room: &Room,
        connection: u64,
    ) -> (Vec<u8>, mpsc::UnboundedReceiver<Outgoing>) {
        let (sender, mut inbox) = outbox();
        room.subscribe(connection, sender, &TextDocument::new().version())
            .await
            .unwrap();
        let Outgoing::Message(ServerMessage::Subscribed { data, .. }) = next(&mut inbox) else {
            panic!("the subscriber did not get the document");
        };
        (data, inbox)
    }

    #[tokio::test]
    async fn a_subscriber_gets_the_missing_changes_and_the_version() {
        let hub = Hub::default();
        let room = room(&hub, "Proof").await;
        let (sender, mut inbox) = outbox();

        room.subscribe(1, sender, &TextDocument::new().version())
            .await
            .unwrap();

        let Outgoing::Message(ServerMessage::Subscribed {
            document,
            version,
            data,
        }) = next(&mut inbox)
        else {
            panic!("the subscriber did not get the document");
        };
        assert_eq!(document, DOCUMENT);
        let loaded = TextDocument::load(&data).unwrap();
        assert_eq!(loaded.text(), "Proof");
        assert_eq!(loaded.version(), version);
    }

    #[tokio::test]
    async fn an_update_reaches_the_others_and_the_author_gets_an_ack() {
        let hub = Hub::default();
        let room = room(&hub, "Proof").await;
        let (data, mut ann) = subscribed(&room, 1).await;
        let (_, mut bob) = subscribed(&room, 2).await;
        let update = edit_of(&data, "s");
        let saved = Arc::new(Mutex::new(Vec::new()));

        let store = Arc::clone(&saved);
        room.publish(1, 7, update.clone(), |data| async move {
            store.lock().unwrap().push(data);
            Ok(())
        })
        .await
        .unwrap();

        assert!(matches!(
            next(&mut ann),
            Outgoing::Message(ServerMessage::Ack {
                document: DOCUMENT,
                id: 7
            })
        ));
        assert!(matches!(
            next(&mut bob),
            Outgoing::Message(ServerMessage::Updates { document: DOCUMENT, data }) if data == update
        ));
        assert_eq!(*saved.lock().unwrap(), [update]);
    }

    #[tokio::test]
    async fn an_update_that_is_not_valid_is_refused_and_not_stored() {
        let hub = Hub::default();
        let room = room(&hub, "Proof").await;
        let (_, mut ann) = subscribed(&room, 1).await;
        let (_, mut bob) = subscribed(&room, 2).await;

        let saved = Arc::new(Mutex::new(Vec::new()));

        let store = Arc::clone(&saved);
        let refused = room
            .publish(1, 7, b"garbage".to_vec(), |data| async move {
                store.lock().unwrap().push(data);
                Ok(())
            })
            .await;

        assert!(matches!(refused, Err(HubError::Sync(_))));
        assert!(saved.lock().unwrap().is_empty());
        assert!(ann.try_recv().is_err());
        assert!(bob.try_recv().is_err());
    }

    #[tokio::test]
    async fn a_storage_failure_ends_all_subscriptions_and_the_room_loads_again() {
        let hub = Hub::default();
        let room = room(&hub, "Proof").await;
        let (data, mut ann) = subscribed(&room, 1).await;
        let (_, mut bob) = subscribed(&room, 2).await;

        let failed = room
            .publish(1, 7, edit_of(&data, "s"), |_| async {
                Err(DocumentError::Database(sqlx::Error::PoolTimedOut))
            })
            .await;

        assert!(matches!(failed, Err(HubError::Store(_))));
        assert!(matches!(next(&mut ann), Outgoing::Close));
        assert!(matches!(next(&mut bob), Outgoing::Close));
        let again = hub
            .room(DOCUMENT, || async { Ok(stored("Proof")) })
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&room, &again));
        let (data, _) = subscribed(&again, 1).await;
        assert_eq!(TextDocument::load(&data).unwrap().text(), "Proof");
    }

    const PRESS: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_00a1);

    fn chat_message(text: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::now_v7(),
            organization: PRESS,
            author: Uuid::nil(),
            author_name: "Ann".to_owned(),
            text: text.to_owned(),
            sent_at: jiff::Timestamp::UNIX_EPOCH,
        }
    }

    #[tokio::test]
    async fn joining_the_chat_gives_the_recent_messages() {
        let hub = Hub::default();
        let (sender, mut inbox) = outbox();
        let recent = vec![chat_message("Ready.")];
        let copy = recent.clone();

        hub.join(PRESS, 1, sender, || async move { Ok(copy) })
            .await
            .unwrap();

        assert!(matches!(
            next(&mut inbox),
            Outgoing::Message(ServerMessage::Joined { organization: PRESS, messages }) if messages == recent
        ));
    }

    #[tokio::test]
    async fn a_chat_message_reaches_each_member_and_the_author() {
        let hub = Hub::default();
        let (ann, mut ann_inbox) = outbox();
        let (bob, mut bob_inbox) = outbox();
        hub.join(PRESS, 1, ann, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        hub.join(PRESS, 2, bob, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        next(&mut ann_inbox);
        next(&mut bob_inbox);
        let message = chat_message("Ready.");

        hub.announce(message.clone()).await;

        for inbox in [&mut ann_inbox, &mut bob_inbox] {
            assert!(matches!(
                next(inbox),
                Outgoing::Message(ServerMessage::Said { message: said }) if said == message
            ));
        }
    }

    #[tokio::test]
    async fn a_message_for_an_organization_reaches_only_its_members() {
        let hub = Hub::default();
        let (ann, mut ann_inbox) = outbox();
        let (eve, mut eve_inbox) = outbox();
        let other = Uuid::from_u128(99);
        hub.join(PRESS, 1, ann, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        hub.join(other, 2, eve, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        next(&mut ann_inbox);
        next(&mut eve_inbox);
        let message = ServerMessage::Error {
            code: "test".to_owned(),
            message: "Only for Field Press.".to_owned(),
        };

        hub.tell(PRESS, &message).await;

        assert!(matches!(next(&mut ann_inbox), Outgoing::Message(told) if told == message));
        assert!(eve_inbox.try_recv().is_err());
    }

    #[tokio::test]
    async fn a_member_who_left_the_chat_gets_no_messages() {
        let hub = Hub::default();
        let (ann, mut inbox) = outbox();
        hub.join(PRESS, 1, ann, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        next(&mut inbox);

        hub.leave_chat(PRESS, 1).await;
        hub.announce(chat_message("Ready.")).await;

        assert!(inbox.try_recv().is_err());
    }

    #[tokio::test]
    async fn the_room_stays_while_it_has_subscribers() {
        let hub = Hub::default();
        let room = room(&hub, "Proof").await;
        subscribed(&room, 1).await;
        subscribed(&room, 2).await;

        let loads = Arc::new(Mutex::new(0));
        let load = || {
            let counter = Arc::clone(&loads);
            || async move {
                *counter.lock().unwrap() += 1;
                Ok(stored("Proof"))
            }
        };

        hub.leave(DOCUMENT, 1).await;
        let same = hub.room(DOCUMENT, load()).await.unwrap();
        assert_eq!(*loads.lock().unwrap(), 0);
        hub.leave(DOCUMENT, 2).await;
        hub.room(DOCUMENT, load()).await.unwrap();

        assert!(Arc::ptr_eq(&room, &same));
        assert_eq!(*loads.lock().unwrap(), 1);
    }
}
