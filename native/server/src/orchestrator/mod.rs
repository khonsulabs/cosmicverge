// TODO this should be refactored into a "PeriodicService" structure that can be reused for driving
//   systems in similar ways to how this piloting one is being set up

use cosmicverge_shared::solar_systems::{universe, Identifiable};
use redis::{self, aio::MultiplexedConnection, RedisError};
use tokio::time::Duration;
use uuid::Uuid;

use crate::{redis::connect_to_redis_multiplex, redis_lock::RedisLock};

pub mod connected_pilots;
pub mod location_store;
mod system_updater;

pub async fn orchestrate() {
    let orchestrator = Orchestrator::new();
    loop {
        match connect_to_redis_multiplex().await {
            // TODO  This should spawn loops for each of these out... at the point we have a connection it will continue to try to reconnect in the background, so it can be its own loop
            Ok(connection) => {
                tokio::spawn(system_updater::run(connection.clone()));
                tokio::spawn(location_store::LocationStore::initialize(
                    connection.clone(),
                ));
                let c2 = connection.clone();
                match tokio::try_join!(orchestrator.run(connection), connected_pilots::run(c2),) {
                    Ok(_) => unreachable!(),
                    Err(err) => {
                        error!("orchestrator redis error while orchestrating {:?}", err);
                    }
                }
            }
            Err(err) => {
                error!("orchestrator error connecting to redis {:?}", err);

                // Unlike above, we want to sleep since we can only get here when we fail to connect
                // We'll throttle reconnect attempts when a connection attempt fails, but won't throttle
                // if we have a single error pop upwards.
                tokio::time::sleep(Duration::from_millis(250)).await
            }
        }
    }
}

#[derive(Debug)]
struct Orchestrator {
    id: Uuid,
}

impl Orchestrator {
    fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    async fn run(&self, mut connection: MultiplexedConnection) -> Result<(), RedisError> {
        loop {
            // Each solar system just needs to update once a second. This loop needs to be stable
            // and update once a second, but there's no guarantee that ticks will line up between systems
            if RedisLock::named("system_queuer")
                .expire_after_msecs(1000)
                .acquire(&mut connection)
                .await?
            {
                // Get the server time and the world timestamp incremented by one
                let (server_timestamp, nanoseconds): (i64, u32) =
                    redis::cmd("TIME").query_async(&mut connection).await?;

                let current_timestamp = server_timestamp as f64 + (nanoseconds as f64 / 1_000_000.);

                // Insert all the IDs into a set, and then publish a notification saying there is stuff to do
                let mut pipe = redis::pipe();
                let mut pipe = &mut pipe;
                pipe = pipe.cmd("SADD").arg("systems_to_process");
                for system_id in universe().systems().map(|s| s.id.id()) {
                    pipe = pipe.arg(system_id);
                }
                pipe = pipe.ignore();

                // Publish the notification to the workers that will process the set as a queue
                pipe = pipe
                    .cmd("PUBLISH")
                    .arg("systems_ready_to_process")
                    .arg(current_timestamp.to_string());

                pipe.query_async(&mut connection).await?;
                info!("Queued systems for update, timestamp {}", current_timestamp);
            }
            tokio::time::sleep(Duration::from_millis(10)).await
        }
    }
}
