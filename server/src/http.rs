use std::{convert::Infallible, path::PathBuf};

use database::cosmicverge_shared::current_git_revision;
use uuid::Uuid;
use warp::{Filter, Reply};

/// parsing support for JSON Web Keys
mod jwk;
/// the websocket api logic
pub mod server;
/// twitch oauth support
mod twitch;

#[cfg(debug_assertions)]
pub fn static_folder() -> PathBuf {
    base_dir().parent().unwrap().join("web").join("static")
}

#[cfg(not(debug_assertions))]
pub fn static_folder() -> PathBuf {
    base_dir().join("static")
}

#[cfg(debug_assertions)]
pub fn base_dir() -> PathBuf {
    let base_dir = std::env!("CARGO_MANIFEST_DIR");
    PathBuf::from(base_dir)
}

#[cfg(not(debug_assertions))]
pub fn base_dir() -> PathBuf {
    std::env::current_dir().unwrap()
}

#[cfg(debug_assertions)]
pub fn webserver_base_url() -> warp::http::uri::Builder {
    warp::http::uri::Uri::builder()
        .scheme("http")
        .authority("localhost:7879")
}

#[cfg(not(debug_assertions))]
pub fn webserver_base_url() -> warp::http::uri::Builder {
    warp::http::uri::Uri::builder()
        .scheme("https")
        .authority("cosmicverge.com")
}

pub async fn run_webserver() -> anyhow::Result<()> {
    info!("server starting up - rev {}", current_git_revision!());

    info!("connecting to database");
    database::initialize().await;
    database::migrations::run_all()
        .await
        .expect("Error running migrations");

    info!("Done running migrations");
    let websocket_server = server::initialize();
    let notify_server = websocket_server.clone();

    crate::redis::initialize().await;

    tokio::spawn(async {
        crate::pubsub::pg_notify_loop(notify_server)
            .await
            .expect("Error on pubsub thread")
    });

    tokio::spawn(crate::orchestrator::orchestrate());

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

    let static_path = static_folder();
    let index_path = static_path.join("bootstrap.html");
    if !static_path.join("pkg").join("web_bg.wasm").exists() {
        error!("Missing WASM app resources. Run `cargo make build` inside of the web crate");
    }

    #[cfg(debug_assertions)]
    let index_handler = warp::get().map(move || {
        // To make the cache expire in debug mode, we're going to always change CACHEBUSTER in the file
        let contents = std::fs::read(&index_path).unwrap();
        let contents = String::from_utf8(contents).unwrap();
        let contents = contents.replace("CACHEBUSTER", &Uuid::new_v4().to_string());
        warp::reply::with_header(contents, "Content-Type", "text/html").into_response()
    });
    #[cfg(not(debug_assertions))]
    let index_handler = warp::fs::file(index_path);

    let spa = warp::get().and(warp::fs::dir(static_path).or(index_handler));

    warp::serve(api.or(healthcheck).or(spa).with(custom_logger))
        .run(([0, 0, 0, 0], 7879))
        .await;

    Ok(())
}

async fn healthcheck() -> Result<impl Reply, Infallible> {
    Ok("ok")
}
