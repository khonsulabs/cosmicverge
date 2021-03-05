use std::time::Duration;

use cosmicverge_shared::{
    num_traits::FromPrimitive,
    protocol::navigation,
    solar_system_simulation::Simulation,
    solar_systems::{universe, SystemId},
};
use futures::StreamExt as _;
use redis::aio::{Connection, MultiplexedConnection};

use crate::{
    orchestrator::location_store::LocationStore,
    redis::{connect_to_redis, RedisLock},
};

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
    let mut last_systems_processed: Option<Vec<i64>> = None;
    while let Some(message) = stream.next().await {
        let current_timestamp: f64 = message.get_payload()?;

        debug!(
            "waking up system_updater - world timestamp {}",
            current_timestamp
        );

        LocationStore::refresh().await?;

        universe().update_orbits(current_timestamp);

        let mut systems_processed = Vec::new();

        loop {
            let (system_ids, last_timestamp): (Vec<i64>, Option<f64>) = redis::pipe()
                .cmd("SRANDMEMBER")
                .arg("systems_to_process")
                // TODO magic number needs to be a configuration
                .arg(3)
                .cmd("GET")
                .arg("world_timestamp")
                .query_async(&mut shared_connection)
                .await?;
            if system_ids.is_empty() {
                break;
            }

            let system_ids = if let Some(mut systems_to_try) = last_systems_processed.take() {
                // If this is our first loop, try to grab the systems we worked on last loop
                // This will reduce lock contention overall, making it happen mostly when nodes
                // are added or lost
                last_systems_processed = None;
                systems_to_try.extend(system_ids);
                systems_to_try
            } else {
                system_ids
            };

            for system_id in system_ids {
                if RedisLock::new(format!("system_update_{}", system_id))
                    .expire_after_msecs(20)
                    .acquire(&mut shared_connection)
                    .await?
                {
                    // Process server update
                    systems_processed.push(system_id);
                    let system = universe()
                        .get(&SystemId::from_i64(system_id).expect("invalid solar system id"));
                    debug!("updating {:?}", system.id);

                    let mut simulation = Simulation::new(system.id, current_timestamp);
                    // TODO limit to pilots *connected*
                    let pilots_in_system = LocationStore::pilots_in_system(system.id).await;

                    simulation.add_ships(pilots_in_system);
                    if let Some(last_timestamp) = last_timestamp {
                        simulation.step((current_timestamp - last_timestamp) as f32);
                    }

                    let mut pipe = redis::pipe();
                    let mut pipe = &mut pipe;

                    for ship in simulation.all_ships() {
                        let location = navigation::Pilot {
                            system: ship.physics.system,
                            location: navigation::System::InSpace(ship.physics.location),
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

        last_systems_processed = Some(systems_processed);

        LocationStore::refresh().await?;

        if RedisLock::named("system_update_completed")
            .expire_after_msecs(900)
            .acquire(&mut shared_connection)
            .await?
        {
            redis::pipe()
                .cmd("PUBLISH")
                .arg("system_update_complete")
                .arg(current_timestamp.to_string())
                .cmd("SET")
                .arg("world_timestamp")
                .arg(current_timestamp)
                .query_async(&mut shared_connection)
                .await?;
        }
    }

    Ok(())
}
