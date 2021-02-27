mod api;
mod cache;
mod database;
mod main_window;

type CosmicVergeClient = basws_client::Client<api::ApiClient>;

use std::path::PathBuf;

use structopt::StructOpt;
use tracing_subscriber::prelude::*;

#[macro_use]
extern crate tracing;

#[cfg(debug_assertions)]
const SERVER_URL: &str = "ws://localhost:7879/v1/ws";
#[cfg(not(debug_assertions))]
const SERVER_URL: &str = "wss://cosmicverge.com/v1/ws";

fn main() -> anyhow::Result<()> {
    let opt = CliOptions::from_args();
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::new(
            "wgpu_core=warn,cosmicverge=trace,kludgine=warn",
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .try_init()?;

    database::ClientDatabase::initialize(
        opt.database
            .clone()
            .unwrap_or_else(|| PathBuf::from("cosmicverge.persy")),
    )?;
    main_window::run(opt.server_url.as_deref().unwrap_or(SERVER_URL))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "cosmicverge", about = "The Cosmic Verge game client")]
struct CliOptions {
    /// which server to connect to
    // In debug, default is ws://localhost:7879/v1/ws.
    // In release, default is wss://cosmicverge.com/v1/ws
    server_url: Option<String>,

    /// where to store the client data
    // default is `./cosmicverge.persy`
    // TODO in release mode, it should store it in the proper location in the user's home folder
    database: Option<PathBuf>,
}
