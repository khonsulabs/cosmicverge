use std::sync::atomic::{AtomicUsize, Ordering};

use cosmicverge_shared::protocol;
use database::{
    basws_server::{self, prelude::Uuid, Handle},
    cosmicverge_shared::protocol::Response,
    schema::{convert_db_pilots, Pilot},
};
use futures::StreamExt as _;
use redis::{aio::Connection, AsyncCommands};
use tokio::time::Duration;

use crate::{
    http::server::{ConnectedAccount, Server},
    orchestrator::location_store::LocationStore,
    redis::connect,
};

pub async fn pg_notify_loop(websockets: basws_server::Server<Server>) -> Result<(), anyhow::Error> {
    loop {
        match connect().await {
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
    websockets: &basws_server::Server<Server>,
) -> Result<(), anyhow::Error> {
    let mut pubsub = connection.into_pubsub();
    pubsub.subscribe("installation_login").await?;
    pubsub.subscribe("connected_pilots_count").await?;
    pubsub.subscribe("system_update_complete").await?;
    let mut stream = pubsub.on_message();
    while let Some(message) = stream.next().await {
        let payload: String = message.get_payload()?;
        debug!(
            "Got notification: {} {}",
            message.get_channel_name(),
            payload
        );
        match message.get_channel_name() {
            // TODO replace magic strings with constants
            "installation_login" => {
                // The payload is the installation_id that logged in.
                let installation_id = Uuid::parse_str(&payload)?;
                if let Ok(account) = ConnectedAccount::lookup(installation_id).await {
                    let response_account = protocol::Account {
                        id: account.account.id,
                        permissions: account.account.permissions(database::pool()).await?,
                    };
                    websockets
                        .associate_installation_with_account(installation_id, Handle::new(account))
                        .await?;

                    let pilots = convert_db_pilots(
                        Pilot::list_by_account_id(response_account.id, database::pool()).await?,
                    );
                    websockets
                        .send_to_installation_id(
                            installation_id,
                            Response::Authenticated {
                                account: response_account,
                                pilots,
                            },
                        )
                        .await;
                }
            }
            "connected_pilots_count" => {
                let connected_pilots: usize = payload.parse()?;
                CONNECTED_CLIENTS.store(connected_pilots, Ordering::Relaxed);
                websockets
                    .broadcast(Response::ServerStatus { connected_pilots })
                    .await;
            }
            "system_update_complete" => {
                let timestamp: f64 = payload.parse()?;

                let system_updates = LocationStore::pilots_by_system().await;
                // This forces the async move to move a reference, not the hash itself
                let system_updates = &system_updates;
                // Iterate over all of the connected clients in parallel
                futures::future::join_all(websockets.connected_clients().await.into_iter().map(
                    |client| async move {
                        // Only send updates to connected pilots
                        if let Some(pilot_id) =
                            client.map_client(|c| c.as_ref().map(Pilot::id)).await
                        {
                            let cache = LocationStore::lookup(pilot_id).await;
                            drop(
                                client
                                    .send_response(Response::SpaceUpdate {
                                        ships: system_updates
                                            .get(&cache.location.system)
                                            .cloned()
                                            .unwrap_or_default(),
                                        location: cache.location,
                                        action: cache.action,
                                        timestamp,
                                    })
                                    .await,
                            );
                        }
                    },
                ))
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

pub async fn notify<S: Into<String> + Send>(
    channel: &'static str,
    payload: S,
) -> Result<(), redis::RedisError> {
    let mut redis = crate::redis::redis().await.clone();
    redis.publish(channel, payload.into()).await
}
