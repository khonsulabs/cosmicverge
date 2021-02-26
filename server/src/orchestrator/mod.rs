// TODO this should be refactored into a "PeriodicService" structure that can be reused for driving
//   systems in similar ways to how this piloting one is being set up

use cosmicverge_shared::solar_systems::{universe, Identifiable};
use redis::{self, aio::MultiplexedConnection};
use tokio::time::Duration;
use uuid::Uuid;

use crate::redis::{connect_to_redis_multiplex, RedisLock};

/// manages tracking and timing out connected pilots
pub mod connected_pilots;
/// manages loading and saving all pilot location data
pub mod location_store;
/// manages running queued updates for systems
mod system_updater;

/// launches the orchestrator and all dependencies
pub async fn orchestrate() {
    let orchestrator = Orchestrator::new();
    loop {
        // The redis library has a MultiplexedConnection which manages
        // automatically reconnecting and can be cloned. For all operations
        // except pubsub, this is perfect, so our outer loop purely exists to
        // establish an initial connection.
        match connect_to_redis_multiplex().await {
            Ok(connection) => {
                // This is separate because of a different function signature.
                // However, it checks to make sure it hasn't already been
                // initialized, so it's harmless if it runs multiple times
                tokio::spawn(location_store::LocationStore::initialize(
                    connection.clone(),
                ));
                let c2 = connection.clone();
                match tokio::try_join!(
                    system_updater::run(connection.clone()),
                    orchestrator.run(connection),
                    connected_pilots::run(c2),
                ) {
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

    ///
    async fn run(&self, mut connection: MultiplexedConnection) -> Result<(), anyhow::Error> {
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
