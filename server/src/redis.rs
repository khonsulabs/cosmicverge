use once_cell::sync::OnceCell;
use redis::aio::MultiplexedConnection;

/// a helper type making it easier to read locking code
mod lock;

pub use lock::RedisLock;

pub async fn initialize() {
    info!("Connecting to redis");
    let redis = connect_to_redis_multiplex().await.unwrap();
    let _ = SHARED_REDIS_CONNECTION.set(redis);
}

static SHARED_REDIS_CONNECTION: OnceCell<MultiplexedConnection> = OnceCell::new();
pub async fn redis() -> &'static MultiplexedConnection {
    SHARED_REDIS_CONNECTION
        .get()
        .expect("use of redis() before initialized")
}

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
