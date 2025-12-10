use std::collections::HashSet;
use std::time::SystemTime;

use midix::prelude::Key;

#[derive(Hash, Eq, PartialEq)]
pub struct KeyWithTimestamp {
    key: Key,
    timestamp: SystemTime,
}

pub struct ActiveKeysHistory {
    history: HashSet<KeyWithTimestamp>,
}

impl ActiveKeysHistory {
    pub fn new() -> Self {
        Self {
            history: HashSet::new(),
        }
    }

    pub fn insert(&mut self, key: Key) {
        self.history.insert(KeyWithTimestamp {
            key,
            timestamp: SystemTime::now(),
        });
    }

    pub fn autoclean(&mut self) {
        self.history
            .retain(|e| e.timestamp.elapsed().is_ok_and(|v| v.as_secs() < 1));
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub fn get(&self) -> HashSet<Key> {
        self.history.iter().map(|e| e.key).collect()
    }

    pub fn len(&self) -> usize {
        self.get().len()
    }
}
