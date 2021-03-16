use chrono::{DateTime, Utc};
use migrations::pool;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, sqlx::Type)]
#[sqlx(type_name = "log_level", rename_all = "lowercase")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Log {
    pub level: Level,
    pub process: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
}

impl Log {
    pub async fn insert_batch(entries: &[Self]) -> sqlx::Result<()> {
        let mut tx = pool().begin().await?;
        for entry in entries {
            sqlx::query!(
                "INSERT INTO logs (level, process, message, timestamp, payload) VALUES ($1, $2, $3, $4, $5)", 
                entry.level as Level,
                entry.process,
                entry.message,
                entry.timestamp,
                entry.payload
            ).execute(&mut tx).await?;
        }

        tx.commit().await
    }

    pub async fn list_recent(count: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT level as \"level: Level\", process, message, timestamp, payload FROM logs ORDER BY timestamp DESC LIMIT $1",
            count
        ).fetch_all(pool()).await
    }
}

#[tokio::test]
async fn test_insert_list() -> sqlx::Result<()> {
    crate::test_util::initialize_exclusive_test().await;
    let logs = vec![Log {
        level: Level::Warning,
        process: String::from("test-process"),
        message: String::from("test-message"),
        timestamp: Utc::now(),
        payload: Some(serde_json::json!({"test": "value"})),
    }];
    Log::insert_batch(&logs).await?;
    let from_db = Log::list_recent(10).await?;

    assert_eq!(logs.len(), from_db.len());
    assert_eq!(logs[0].level, from_db[0].level);
    assert_eq!(logs[0].process, from_db[0].process);
    assert_eq!(logs[0].message, from_db[0].message);
    // This avoids the floating point error from Eq/PartialEq and the round trip
    // from the database.
    assert_eq!(
        logs[0].timestamp.timestamp(),
        from_db[0].timestamp.timestamp()
    );
    assert_eq!(logs[0].payload, from_db[0].payload);

    Ok(())
}
