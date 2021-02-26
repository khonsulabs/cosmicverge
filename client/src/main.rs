mod api;
mod cache;
mod database;
mod main_window;

type CosmicVergeClient = basws_client::Client<api::ApiClient>;

use basws_client::Url;
use structopt::StructOpt;
use tracing_subscriber::prelude::*;

#[macro_use]
extern crate tracing;

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

    database::ClientDatabase::initialize("cosmicverge.sleddb")?;
    main_window::run()
}

#[derive(StructOpt, Debug)]
#[structopt(name = "cosmicverge", about = "The Cosmic Verge game client")]
struct CliOptions {}
