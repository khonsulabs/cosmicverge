#[macro_use]
extern crate tracing;

use std::{convert::Infallible, path::Path};

use database::cosmicverge_shared::current_git_revision;
use once_cell::sync::OnceCell;
use redis::aio::MultiplexedConnection;
use warp::{Filter, Reply};

mod jwk;
mod orchestrator;
mod pubsub;
mod redis_lock;
mod server;
mod twitch;

#[cfg(debug_assertions)]
const STATIC_FOLDER_PATH: &str = "../../web/static";
#[cfg(not(debug_assertions))]
const STATIC_FOLDER_PATH: &str = "static";

#[cfg(debug_assertions)]
const PRIVATE_ASSETS_PATH: &str = "../../private/assets";

#[cfg(debug_assertions)]
fn webserver_base_url() -> warp::http::uri::Builder {
    warp::http::uri::Uri::builder()
        .scheme("http")
        .authority("localhost:7879")
}

#[cfg(not(debug_assertions))]
fn webserver_base_url() -> warp::http::uri::Builder {
    warp::http::uri::Uri::builder()
        .scheme("https")
        .authority("cosmicverge.com")
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Error initializing environment");
    initialize_logging();
    info!("server starting up - rev {}", current_git_revision!());

    info!("connecting to database");
    database::initialize().await;
    database::migrations::run_all()
        .await
        .expect("Error running migrations");

    info!("Done running migrations");
    let websocket_server = server::initialize();
    let notify_server = websocket_server.clone();

    info!("Connecting to redis");
    let redis = connect_to_redis_multiplex().await.unwrap();
    let _ = SHARED_REDIS_CONNECTION.set(redis);

    tokio::spawn(async {
        pubsub::pg_notify_loop(notify_server)
            .await
            .expect("Error on pubsub thread")
    });

    tokio::spawn(orchestrator::orchestrate());

    let auth = twitch::callback();
    let websocket_route = warp::path!("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let websocket_server = websocket_server.clone();
            ws.on_upgrade(|ws| async move { websocket_server.incoming_connection(ws).await })
        });
    let api = warp::path("v1").and(websocket_route.or(auth));

    let custom_logger = warp::log::custom(|info| {
        if info.status().is_server_error() {
            error!(
                path = info.path(),
                method = info.method().as_str(),
                status = info.status().as_str(),
                "Request Served"
            );
        } else {
            info!(
                path = info.path(),
                method = info.method().as_str(),
                status = info.status().as_u16(),
                "Request Served"
            );
        }
    });

    let healthcheck = warp::get()
        .and(warp::path("__healthcheck"))
        .and_then(healthcheck);

    let base_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    let base_dir = Path::new(&base_dir);
    let static_path = base_dir.join(STATIC_FOLDER_PATH);
    let index_path = static_path.join("index.html");

    let spa = warp::get().and(warp::fs::dir(static_path).or(warp::fs::file(index_path)));

    #[cfg(debug_assertions)]
    let routes = {
        let private_assets_path = base_dir.join(PRIVATE_ASSETS_PATH);
        let private_assets = warp::fs::dir(private_assets_path);

        private_assets.or(api)
    };
    #[cfg(not(debug_assertions))]
    let routes = api;

    warp::serve(routes.or(healthcheck).or(spa).with(custom_logger))
        .run(([0, 0, 0, 0], 7879))
        .await
}

async fn healthcheck() -> Result<impl Reply, Infallible> {
    Ok("ok")
}

fn env(var: &str) -> String {
    std::env::var(var).unwrap()
}

pub fn initialize_logging() {
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .try_init()
        .unwrap();
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
