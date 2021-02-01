#[macro_use]
extern crate log;

use chrono::Utc;
pub use redis;
use redis::{aio::MultiplexedConnection, RedisError};
use tokio::time::Duration;
use uuid::Uuid;

use crate::{redis::AsyncCommands, redis_lock::RedisLock};

pub mod connected_pilots;
mod redis_lock;

pub async fn connect_to_redis_multiplex(
) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
    redis::Client::open(std::env::var("REDIS_URL").expect("REDIS_URL not found"))
        .unwrap()
        .get_multiplexed_tokio_connection()
        .await
}

pub async fn connect_to_redis() -> Result<redis::aio::Connection, redis::RedisError> {
    redis::Client::open(std::env::var("REDIS_URL").expect("REDIS_URL not found"))
        .unwrap()
        .get_tokio_connection()
        .await
}

pub async fn orchestrate() {
    let orchestrator = Orchestrator::new();
    loop {
        match connect_to_redis_multiplex().await {
            Ok(connection) => {
                let c2 = connection.clone();
                match tokio::try_join!(
                    orchestrator.run(connection),
                    connected_pilots::manager_loop(c2)
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

    async fn run(&self, mut connection: MultiplexedConnection) -> Result<(), RedisError> {
        loop {
            let now = Utc::now();
            connection
                .hset("orchestrators", self.id.to_string(), now.timestamp())
                .await?;

            // Each solar system just needs to update once a second. This loop needs to be stable
            // and update once a second, but there's no guarantee that ticks will line up between systems
            if RedisLock::named("system_queuer")
                .expire_after_msecs(9997)
                .acquire(&mut connection)
                .await?
            {}

            // Picked a prime number below 100 to try to ensure one server will hit the lock at almost exactly the moment it is freed
            tokio::time::sleep(Duration::from_millis(97)).await
        }
    }
}
