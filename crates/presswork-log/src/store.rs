use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use futures::channel::mpsc;

use crate::LogEntry;

#[derive(Debug, Clone)]
pub struct LogStore {
    shared: Arc<Mutex<Shared>>,
}

#[derive(Debug)]
struct Shared {
    capacity: usize,
    entries: VecDeque<LogEntry>,
    last_sequence: u64,
    watchers: Vec<mpsc::Sender<()>>,
}

impl LogStore {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            shared: Arc::new(Mutex::new(Shared {
                capacity,
                entries: VecDeque::with_capacity(capacity),
                last_sequence: 0,
                watchers: Vec::new(),
            })),
        }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.lock().capacity
    }

    pub fn push(&self, mut entry: LogEntry) {
        let mut shared = self.lock();
        shared.last_sequence += 1;
        entry.sequence = shared.last_sequence;
        if shared.entries.len() == shared.capacity {
            shared.entries.pop_front();
        }
        shared.entries.push_back(entry);
        shared
            .watchers
            .retain_mut(|watcher| match watcher.try_send(()) {
                Ok(()) => true,
                Err(error) => error.is_full(),
            });
    }

    #[must_use]
    pub fn after(&self, sequence: u64) -> Vec<LogEntry> {
        let shared = self.lock();
        let start = shared
            .entries
            .partition_point(|entry| entry.sequence <= sequence);
        shared.entries.range(start..).cloned().collect()
    }

    #[must_use]
    pub fn watch(&self) -> mpsc::Receiver<()> {
        let (sender, receiver) = mpsc::channel(0);
        self.lock().watchers.push(sender);
        receiver
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Hash for LogStore {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.shared).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::hash::{BuildHasher, RandomState};

    fn entry(message: &str) -> LogEntry {
        LogEntry {
            message: message.to_owned(),
            ..LogEntry::default()
        }
    }

    fn messages(entries: Vec<LogEntry>) -> Vec<String> {
        entries.into_iter().map(|entry| entry.message).collect()
    }

    #[test]
    fn push_gives_increasing_sequence_numbers() {
        let store = LogStore::with_capacity(10);

        store.push(entry("a"));
        store.push(entry("b"));

        let sequences: Vec<u64> = store.after(0).iter().map(|entry| entry.sequence).collect();
        assert_eq!(sequences, [1, 2]);
    }

    #[test]
    fn after_gives_the_entries_after_a_sequence_number() {
        let store = LogStore::with_capacity(10);
        store.push(entry("a"));
        store.push(entry("b"));

        assert_eq!(messages(store.after(1)), ["b"]);
    }

    #[test]
    fn a_full_store_drops_the_oldest_entry() {
        let store = LogStore::with_capacity(2);
        for message in ["a", "b", "c"] {
            store.push(entry(message));
        }

        assert_eq!(messages(store.after(0)), ["b", "c"]);
    }

    #[test]
    fn a_watcher_gets_a_signal_for_a_new_entry() {
        let store = LogStore::with_capacity(10);
        let mut watcher = store.watch();

        store.push(entry("a"));

        assert_eq!(futures::executor::block_on(watcher.next()), Some(()));
    }

    #[test]
    fn a_closed_watcher_does_not_stop_new_entries() {
        let store = LogStore::with_capacity(10);
        drop(store.watch());

        store.push(entry("a"));

        assert_eq!(messages(store.after(0)), ["a"]);
    }

    #[test]
    fn clones_share_the_entries_and_the_hash() {
        let store = LogStore::with_capacity(10);
        let clone = store.clone();
        let hasher = RandomState::new();

        clone.push(entry("a"));

        assert_eq!(messages(store.after(0)), ["a"]);
        assert_eq!(hasher.hash_one(&store), hasher.hash_one(&clone));
    }
}
