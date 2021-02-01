#[macro_use]
extern crate log;

pub use redis;
use redis::{aio::MultiplexedConnection, RedisError};
use tokio::time::Duration;

pub mod connected_pilots;

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
    loop {
        match connect_to_redis_multiplex().await {
            Ok(connection) => {
                let c2 = connection.clone();
                match tokio::try_join!(
                    orchestrator_loop(connection),
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

async fn orchestrator_loop(_connection: MultiplexedConnection) -> Result<(), RedisError> {
    loop {
        // Each solar system just needs to update once a second. This loop needs to be stable
        // and update once a second, but there's no guarantee that ticks will line up between systems
        tokio::time::sleep(Duration::from_millis(250)).await
    }
}
