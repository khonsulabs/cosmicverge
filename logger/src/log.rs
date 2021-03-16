use std::{fmt::Display, sync::Arc};

use chrono::{DateTime, Utc};
use flume::Sender;
use futures::Future;
use once_cell::sync::OnceCell;
use serde::{de::Error, Deserialize, Serialize};

static GLOBAL_SENDER: OnceCell<Sender<Arc<Log>>> = OnceCell::new();

tokio::task_local! {
    static TASK_SENDER: Option<Sender<Arc<Log>>>;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Debug,
    Eq,
    PartialEq,
    strum_macros::EnumIter,
    strum_macros::Display,
)]
pub enum Pod {
    ClusterControl,
    NodeControl,
    SystemServer,
    ApiServer,
    Client,
}

pub trait Message: Display {
    fn process(&self) -> Pod;
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Log {
    pub level: Level,
    pub process: Pod,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
}

impl Log {
    #[allow(clippy::clippy::needless_pass_by_value)] // This is a choice to make these APIs read cleaner, as Categories are always expected to be an enum constant.
    pub fn new<M: Message>(level: Level, message: M) -> Self {
        Self {
            level,
            process: message.process(),
            message: message.to_string(),
            timestamp: Utc::now(),
            payload: serde_json::Value::Null,
        }
    }

    pub fn error<M: Message>(message: M) -> Self {
        Self::new(Level::Error, message)
    }

    pub fn warning<M: Message>(message: M) -> Self {
        Self::new(Level::Warning, message)
    }

    pub fn info<M: Message>(message: M) -> Self {
        Self::new(Level::Info, message)
    }

    pub fn debug<M: Message>(message: M) -> Self {
        Self::new(Level::Debug, message)
    }

    pub fn trace<M: Message>(message: M) -> Self {
        Self::new(Level::Trace, message)
    }

    pub fn add<K: Into<String>, V: Serialize>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<&mut Self, serde_json::Error> {
        if matches!(self.payload, serde_json::Value::Null) {
            self.payload = serde_json::Value::Object(serde_json::Map::new());
        }
        let key = key.into();
        if self
            .payload
            .as_object_mut()
            .unwrap()
            .insert(key, serde_json::value::to_value(value)?)
            .is_some()
        {
            return Err(serde_json::Error::custom(
                "attempting to add the same key twice",
            ));
        }

        Ok(self)
    }

    pub fn with<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self, serde_json::Error> {
        self.add(key, value)?;

        Ok(self)
    }

    pub fn submit(self) {
        let log = Arc::new(self);
        TASK_SENDER
            .try_with(|sender| sender.as_ref().map(|s| s.send(log.clone())))
            .unwrap_or_else(|_| GLOBAL_SENDER.get().as_ref().map(|sender| sender.send(log)))
            .expect("log submitted with no backend")
            .expect("error sending to log backend");
    }

    pub fn set_global_destination(sender: Sender<Arc<Self>>) {
        // Uninitialize the sender in case it already was initialized
        GLOBAL_SENDER.set(sender).unwrap();
    }

    pub async fn run_with_destination<F: Future<Output = R> + Send, R: Send>(
        sender: Sender<Arc<Self>>,
        future: F,
    ) -> R {
        TASK_SENDER.scope(Some(sender), future).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TestMessage;

    fn entries_eq_without_timestamps(a: &Log, b: &Log) -> bool {
        a.level == b.level
            && a.process == b.process
            && a.message == b.message
            && a.payload == b.payload
    }

    #[test]
    fn entry_building_tests() -> Result<(), serde_json::Error> {
        assert!(entries_eq_without_timestamps(
            &Log::debug(TestMessage::A),
            &Log {
                level: Level::Debug,
                process: TestMessage::A.process(),
                message: String::from("A"),
                payload: serde_json::Value::Null,
                timestamp: Utc::now(),
            }
        ));

        assert!(entries_eq_without_timestamps(
            &Log::info(TestMessage::A),
            &Log {
                level: Level::Info,
                process: TestMessage::A.process(),
                message: String::from("A"),
                payload: serde_json::Value::Null,
                timestamp: Utc::now(),
            }
        ));

        assert!(entries_eq_without_timestamps(
            &Log::warning(TestMessage::B),
            &Log {
                level: Level::Warning,
                process: TestMessage::B.process(),
                message: String::from("B"),
                payload: serde_json::Value::Null,
                timestamp: Utc::now(),
            }
        ));

        assert!(entries_eq_without_timestamps(
            Log::error(TestMessage::B).add("key", "value")?,
            &Log {
                level: Level::Error,
                process: TestMessage::B.process(),
                message: String::from("B"),
                payload: serde_json::json!({"key": "value"}),
                timestamp: Utc::now(),
            }
        ));

        assert!(entries_eq_without_timestamps(
            Log::trace(TestMessage::B)
                .add("key", "value")?
                .add("key2", "value2")?,
            &Log {
                level: Level::Trace,
                process: TestMessage::B.process(),
                message: String::from("B"),
                payload: serde_json::json!({"key": "value", "key2": "value2"}),
                timestamp: Utc::now(),
            }
        ));

        assert!(Log::trace(TestMessage::B)
            .add("key", "value")?
            .add("key", "value")
            .is_err());

        Ok(())
    }
}

#[cfg(feature = "archiver")]
impl Into<database::schema::Log> for Log {
    fn into(self) -> database::schema::Log {
        database::schema::Log {
            level: self.level.into(),
            process: self.process.to_string(),
            message: self.message,
            payload: match self.payload {
                serde_json::Value::Null => None,
                other => Some(other),
            },
            timestamp: self.timestamp,
        }
    }
}

#[cfg(feature = "archiver")]
impl Into<database::schema::Level> for Level {
    fn into(self) -> database::schema::Level {
        match self {
            Self::Error => database::schema::Level::Error,
            Self::Warning => database::schema::Level::Warning,
            Self::Info => database::schema::Level::Info,
            Self::Debug => database::schema::Level::Debug,
            Self::Trace => database::schema::Level::Trace,
        }
    }
}
