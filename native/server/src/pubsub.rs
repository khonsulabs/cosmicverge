use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use cosmicverge_shared::{
    protocol::{PilotedShip, SolarSystemLocation},
    solar_systems::SolarSystemId,
    strum::IntoEnumIterator,
};
use database::{
    basws_server::{prelude::Uuid, Handle, Server},
    cosmicverge_shared::protocol::CosmicVergeResponse,
    schema::{convert_db_pilots, Pilot},
};
use futures::StreamExt as _;
use redis::{aio::Connection, AsyncCommands};
use tokio::time::Duration;

use crate::{
    connect_to_redis,
    orchestrator::location_store::LocationStore,
    server::{ConnectedAccount, CosmicVergeServer},
};

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
            "system_update_complete" => {
                let timestamp: i64 = payload.parse()?;

                let system_updates =
                    futures::future::join_all(SolarSystemId::iter().map(|system_id| async move {
                        (system_id, load_system_ships(system_id).await)
                    }))
                    .await
                    .into_iter()
                    .collect::<HashMap<SolarSystemId, Vec<PilotedShip>>>();

                // This forces the async move to move a reference, not the hash itself
                let system_updates = &system_updates;
                // Iterate over all of the connected clients in parallel
                futures::future::join_all(websockets.connected_clients().await.into_iter().map(
                    |client| async move {
                        // Only send updates to connected pilots
                        if let Some(pilot_id) =
                            client.map_client(|c| c.as_ref().map(|p| p.id())).await
                        {
                            let cache = LocationStore::lookup(pilot_id).await;
                            let _ = client
                                .send_response(CosmicVergeResponse::SpaceUpdate {
                                    ships: system_updates[&cache.location.system].clone(),
                                    location: cache.location,
                                    action: cache.action,
                                    timestamp,
                                })
                                .await;
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

pub async fn notify<S: ToString>(
    channel: &'static str,
    payload: S,
) -> Result<(), redis::RedisError> {
    let mut redis = crate::redis().await.clone();
    redis.publish(channel, payload.to_string()).await
}

async fn load_system_ships(system_id: SolarSystemId) -> Vec<PilotedShip> {
    let system_pilots = LocationStore::pilots_in_system(system_id).await.into_iter();

    futures::future::join_all(system_pilots.map(|pilot_id| async move {
        let cache = LocationStore::lookup(pilot_id).await;

        match cache.location.location {
            SolarSystemLocation::InSpace(_) => Some(PilotedShip {
                pilot_id,
                ship: cache.ship,
                action: cache.action,
                physics: cache.physics,
            }),
            SolarSystemLocation::Docked(_) => None,
        }
    }))
    .await
    .into_iter()
    .filter_map(|s| s)
    .collect()
}
