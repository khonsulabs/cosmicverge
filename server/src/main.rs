#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![cfg_attr(doc, warn(rustdoc))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    // clippy::missing_panics_doc, // not on stable yet
    clippy::option_if_let_else,
)]

#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use structopt::StructOpt;
use tracing_subscriber::prelude::*;

/// the definition of the http server.
mod http;
/// controls the game loop logic
mod orchestrator;
/// defines procedurally generated planets
mod planets;
/// pub-sub message exchanging with other servers
mod pubsub;
/// shared connection pools
mod redis;

/// command line interface support
mod cli;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "cosmicverge",
    about = "Cosmic Verge web server and associated tools"
)]
/// the command line interface for the executable
///
/// For help, try executing `cosmicverge-server -h`. Here are some common commands you might need when getting started:
///
/// - `cosmicverge-server generate-assets <static-folder-path>`: Generates the procedurally generated assets into folder specified.
/// - `cosmicverge-server serve`: Starts the game server
/// - `cosmicverge-server account --id 1 set-super-user`: Sets Account ID 1 to a Superuser.
enum Cli {
    /// Run the Server
    Serve,
    /// Generate static assets, currently just includes procedurally generated planets
    GenerateAssets {
        /// The path to the static folder to generate the assets within
        static_folder: PathBuf,
    },
    Account(cli::accounts::Command),
    PermissionGroup(cli::permission_groups::Command),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Error initializing environment");
    initialize_logging();

    let cli = Cli::from_args();
    match cli {
        Cli::Serve => http::run_webserver().await,
        Cli::GenerateAssets { static_folder } => generate_assets(static_folder).await,
        Cli::Account(command) => command.execute().await,
        Cli::PermissionGroup(command) => command.execute().await,
    }
}

pub fn initialize_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::new(
            "sqlx=warn,cosmicverge-server=trace",
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .try_init()
        .unwrap();
}

async fn generate_assets(static_folder: PathBuf) -> anyhow::Result<()> {
    planets::generate_assets(&static_folder);
    Ok(())
}
