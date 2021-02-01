use std::sync::atomic::{AtomicUsize, Ordering};

use database::{
    basws_server::{prelude::Uuid, Handle, Server},
    cosmicverge_shared::CosmicVergeResponse,
    schema::{convert_db_pilots, Pilot},
};
use futures::StreamExt as _;
use orchestrator::{
    connect_to_redis, redis,
    redis::{aio::Connection, AsyncCommands},
};
use tokio::time::Duration;

use crate::server::{ConnectedAccount, CosmicVergeServer};

pub async fn pg_notify_loop(websockets: Server<CosmicVergeServer>) -> Result<(), anyhow::Error> {
    loop {
        match connect_to_redis().await {
            Ok(connection) => match wait_for_messages(connection, &websockets).await {
                Ok(_) => error!("Redis disconnected processing pubsub"),
                Err(err) => {
                    error!("Error while processing pubsub messages: {:?}", err);
                }
            },
            Err(err) => {
                error!("Error connecting to redis. {:?}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

static CONNECTED_CLIENTS: AtomicUsize = AtomicUsize::new(0);

async fn wait_for_messages(
    connection: Connection,
    websockets: &Server<CosmicVergeServer>,
) -> Result<(), anyhow::Error> {
    let mut pubsub = connection.into_pubsub();
    pubsub.subscribe("installation_login").await?;
    pubsub.subscribe("connected_pilots_count").await?;
    let mut stream = pubsub.on_message();
    while let Some(message) = stream.next().await {
        let payload: String = message.get_payload()?;
        info!(
            "Got notification: {} {}",
            message.get_channel_name(),
            payload
        );
        match message.get_channel_name() {
            "installation_login" => {
                // The payload is the installation_id that logged in.
                let installation_id = Uuid::parse_str(&payload)?;
                if let Ok(account) = ConnectedAccount::lookup(installation_id).await {
                    let user_id = account.account.id;
                    websockets
                        .associate_installation_with_account(installation_id, Handle::new(account))
                        .await?;

                    let pilots = convert_db_pilots(
                        Pilot::list_by_account_id(user_id, database::pool()).await?,
                    );
                    websockets
                        .send_to_installation_id(
                            installation_id,
                            CosmicVergeResponse::Authenticated { user_id, pilots },
                        )
                        .await;
                }
            }
            "connected_pilots_count" => {
                let connected_pilots: usize = payload.parse()?;
                CONNECTED_CLIENTS.store(connected_pilots, Ordering::Relaxed);
                websockets
                    .broadcast(CosmicVergeResponse::ServerStatus { connected_pilots })
                    .await;
            }
            other => error!("Unexpected channel for message: {:?}", other),
        }
    }

    Ok(())
}

pub fn connected_pilots_count() -> usize {
    CONNECTED_CLIENTS.load(Ordering::Relaxed)
}

pub async fn notify<S: ToString>(
    channel: &'static str,
    payload: S,
) -> Result<(), redis::RedisError> {
    let mut redis = crate::redis().await.clone();
    redis.publish(channel, payload.to_string()).await
}
