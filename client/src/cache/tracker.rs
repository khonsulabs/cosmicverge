use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

#[derive(Debug)]
pub struct Tracker<T> {
    alive: HashMap<String, Weak<T>>,
}

impl<T> Default for Tracker<T> {
    fn default() -> Self {
        Self {
            alive: HashMap::new(),
        }
    }
}

impl<T> Tracker<T> {
    pub fn lookup(&self, key: &str) -> Option<Arc<T>> {
        if let Some(entry) = self.alive.get(key) {
            if let Some(entry) = entry.upgrade() {
                return Some(entry);
            }
        }

        None
    }

    pub fn track<F: FnOnce() -> T>(&mut self, key: String, initializer: F) -> Arc<T> {
        if let Some(entry) = self.lookup(&key) {
            return entry;
        }

        // Otherwise, initialize it
        let value = Arc::new(initializer());
        let entry = Arc::downgrade(&value);
        self.alive.insert(key, entry);
        value
    }
}
