use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use cosmicverge_shared::protocol::PilotId;
use once_cell::sync::OnceCell;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::redis_lock::RedisLock;

#[derive(Serialize, Deserialize, Debug)]
struct ConnectedPilotInfo {
    connected_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
}

impl Default for ConnectedPilotInfo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            connected_at: now,
            last_seen_at: now,
        }
    }
}

pub(crate) async fn run(mut connection: MultiplexedConnection) -> Result<(), anyhow::Error> {
    let pilot_reader = connection_channel().1.clone();
    loop {
        let mut new_pilots = HashSet::new();
        while let Ok(pilot_id) = pilot_reader.try_recv() {
            new_pilots.insert(pilot_id);
        }

        if !new_pilots.is_empty() {
            info!("{} pilots connected", new_pilots.len());
            let entries = new_pilots
                .into_iter()
                .map(|pilot_id| {
                    (
                        pilot_id.to_string(),
                        serde_json::to_string(&ConnectedPilotInfo::default()).unwrap(),
                    )
                })
                .collect::<Vec<_>>();

            connection
                .hset_multiple("connected_pilots", &entries)
                .await?;
        }

        if RedisLock::named("connected_pilots_cleaner")
            .expire_after_secs(30)
            .acquire(&mut connection)
            .await?
        {
            let mut disconnected = HashSet::new();
            let cutoff = Utc::now() - chrono::Duration::minutes(1);
            let connected_pilots: HashMap<PilotId, String> =
                connection.hgetall("connected_pilots").await?;
            for (pilot_id, payload) in connected_pilots {
                if let Ok(info) = serde_json::from_str::<ConnectedPilotInfo>(&payload) {
                    if info.last_seen_at > cutoff {
                        continue;
                    }
                }

                disconnected.insert(pilot_id.to_string());
            }

            if !disconnected.is_empty() {
                info!("{} pilots disconnected", disconnected.len());

                let mut args = Vec::with_capacity(disconnected.len() + 1);
                args.push(String::from("connected_pilots"));
                args.extend(disconnected);
                redis::cmd("HDEL")
                    .arg(args.as_slice())
                    .query_async(&mut connection)
                    .await?
            }
        }

        if RedisLock::named("connected_pilots_counter")
            .expire_after_secs(5)
            .acquire(&mut connection)
            .await?
        {
            let connected_pilots_count: usize = connection.hlen("connected_pilots").await?;
            connection
                .publish("connected_pilots_count", connected_pilots_count)
                .await?;
        }

        tokio::time::sleep(Duration::from_secs(1)).await
    }
}

fn connection_channel() -> &'static (
    async_channel::Sender<PilotId>,
    async_channel::Receiver<PilotId>,
) {
    static REUSED_CHANNEL: OnceCell<(
        async_channel::Sender<PilotId>,
        async_channel::Receiver<PilotId>,
    )> = OnceCell::new();
    REUSED_CHANNEL.get_or_init(async_channel::unbounded)
}

pub async fn note(pilot_id: PilotId) {
    let _ = connection_channel().0.send(pilot_id).await;
}
