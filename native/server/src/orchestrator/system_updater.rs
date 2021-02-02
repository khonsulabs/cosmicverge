use std::time::Duration;

use cosmicverge_shared::{
    num_traits::FromPrimitive,
    solar_systems::{universe, SolarSystemId},
};
use futures::StreamExt as _;
use redis::{
    aio::{Connection, MultiplexedConnection},
    AsyncCommands,
};

use crate::{connect_to_redis, orchestrator::location_store::LocationStore, redis_lock::RedisLock};

pub async fn run(shared_connection: MultiplexedConnection) -> Result<(), anyhow::Error> {
    loop {
        match connect_to_redis().await {
            Ok(connection) => {
                match wait_for_ready_to_process(connection, shared_connection.clone()).await {
                    Ok(_) => error!("Redis disconnected processing system updates"),
                    Err(err) => {
                        error!("Error while processing system update messages: {:?}", err);
                    }
                }
            }
            Err(err) => {
                error!("Error connecting to redis. {:?}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn wait_for_ready_to_process(
    pubsub_connection: Connection,
    mut shared_connection: MultiplexedConnection,
) -> Result<(), redis::RedisError> {
    // Add another process that listens for world-to-process messages and clears the queue
    // We probably want multiple per server, but maybe it's a configuration option
    // That queue process
    //   can load all connected pilot data, ask for a random solar system,
    //   acquire a lock for that system (30ms or so?)
    //   run the pilot update for that
    let mut pubsub = pubsub_connection.into_pubsub();
    pubsub.subscribe("systems_ready_to_process").await?;
    let mut stream = pubsub.on_message();
    while let Some(message) = stream.next().await {
        let current_timestamp: i64 = message.get_payload()?;

        info!(
            "waking up system_updater - world timestamp {}",
            current_timestamp
        );

        LocationStore::refresh().await?;

        loop {
            // We could ask for multiple members in the same query depending on how we decide to scale
            // By asking for a few random members, we would be more likely to not have to have multiple
            // return trips to redis due to contention. However, it seems like it would be better to
            // have multiple async waiting tasks per worker than it would be to juggle that issue.
            // The benefit of this design is that if a worker crashes mid-update, another will try to update
            // it again a little bit later. Having a system where we pre-assign systems to each server means
            // a server crashing causes a much bigger impact because the rebalance has to happen mid-world-update,
            let system_id: Option<i64> =
                shared_connection.srandmember("systems_to_process").await?;
            match system_id {
                Some(system_id) => {
                    if RedisLock::new(format!("system_update_{}", system_id))
                        .expire_after_msecs(20)
                        .acquire(&mut shared_connection)
                        .await?
                    {
                        // Process server update
                        let system = universe().get(
                            &SolarSystemId::from_i64(system_id).expect("invalid solar system id"),
                        );
                        info!("updating {:?}", system.id);

                        shared_connection
                            .srem("systems_to_process", system_id)
                            .await?;
                    }
                }
                None => break,
            }
        }

        info!("system_updater completed");
    }

    Ok(())
}
