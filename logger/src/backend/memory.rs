use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{backend::Backend, Log};

#[derive(Debug)]
pub struct Memory {
    pub max_entries: usize,
    pub entries: Arc<Mutex<VecDeque<Log>>>,
}

impl Memory {
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Arc::default(),
        }
    }
}

#[async_trait]
impl Backend for Memory {
    async fn process_log(&mut self, log: &Log) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().await;

        entries.push_front(log.clone());

        while entries.len() > self.max_entries {
            entries.pop_back();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{test_util::TestMessage, Manager};

    use super::*;

    #[tokio::test]
    async fn send_test() -> anyhow::Result<()> {
        let test_backend = Memory::new(2);
        let entries = test_backend.entries.clone();
        let sender = Manager::default()
            .with_backend(test_backend)
            .launch(|task| {
                tokio::spawn(task);
            });

        sender.try_send(Arc::new(Log::info(TestMessage::A)))?;
        sender.try_send(Arc::new(Log::info(TestMessage::B)))?;
        sender.try_send(Arc::new(Log::info(TestMessage::A)))?;

        tokio::time::sleep(Duration::from_millis(1)).await;
        {
            let entries = entries.lock().await;
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].message, "A");
            assert_eq!(entries[1].message, "B");
        }

        Ok(())
    }
}
