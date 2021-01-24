use std::path::Path;
use warp::Filter;

#[macro_use]
extern crate tracing;

#[cfg(debug_assertions)]
const STATIC_FOLDER_PATH: &str = "../../web/static";
#[cfg(not(debug_assertions))]
const STATIC_FOLDER_PATH: &str = "static";

#[tokio::main]
async fn main() {
    info!("server starting up");

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

    let base_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    let base_dir = Path::new(&base_dir);
    let static_path = dbg!(base_dir.join(STATIC_FOLDER_PATH));
    let index_path = static_path.join("index.html");

    let spa = warp::get()
        .and(warp::fs::dir(static_path).or(warp::fs::file(index_path)))
        .with(custom_logger);
    let spa_only_server = warp::serve(spa).run(([0, 0, 0, 0], 7879));

    spa_only_server.await
}
