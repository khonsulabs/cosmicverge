use std::time::Duration;

use cosmicverge_shared::{
    num_traits::FromPrimitive,
    protocol::{PilotLocation, SolarSystemLocation},
    solar_system_simulation::SolarSystemSimulation,
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

        debug!(
            "waking up system_updater - world timestamp {}",
            current_timestamp
        );

        LocationStore::refresh().await?;

        universe().update_orbits(current_timestamp);

        loop {
            // TODO magic number needs to be a configuration
            let system_ids: Vec<i64> = shared_connection
                .srandmember_multiple("systems_to_process", 3)
                .await?;
            if system_ids.is_empty() {
                break;
            }

            for system_id in system_ids {
                if RedisLock::new(format!("system_update_{}", system_id))
                    .expire_after_msecs(20)
                    .acquire(&mut shared_connection)
                    .await?
                {
                    // Process server update
                    let system = universe()
                        .get(&SolarSystemId::from_i64(system_id).expect("invalid solar system id"));
                    debug!("updating {:?}", system.id);

                    let mut simulation = SolarSystemSimulation::new(system.id);
                    // TODO limit to pilots *connected*
                    let pilots_in_system = LocationStore::pilots_in_system(system.id).await;

                    simulation.add_ships(pilots_in_system);
                    simulation.step(1.0);

                    let mut pipe = redis::pipe();
                    let mut pipe = &mut pipe;

                    for ship in simulation.all_ships() {
                        let location = PilotLocation {
                            system: ship.physics.system,
                            location: SolarSystemLocation::InSpace(ship.physics.location),
                        };
                        pipe = pipe
                            .cmd("HSET")
                            .arg("pilot_locations")
                            .arg(ship.pilot_id)
                            .arg(serde_json::to_string(&location).unwrap());
                        pipe = pipe
                            .cmd("HSET")
                            .arg("pilot_physics")
                            .arg(ship.pilot_id)
                            .arg(serde_json::to_string(&ship.physics).unwrap());
                    }

                    pipe = pipe.cmd("srem").arg("systems_to_process").arg(system_id);
                    pipe.query_async(&mut shared_connection).await?;
                }
            }
        }

        LocationStore::refresh().await?;

        if RedisLock::named("system_update_completed")
            .expire_after_msecs(900)
            .acquire(&mut shared_connection)
            .await?
        {
            shared_connection
                .publish("system_update_complete", current_timestamp.to_string())
                .await?;
        }
    }

    Ok(())
}
