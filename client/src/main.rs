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
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    // clippy::missing_panics_doc, // not on stable yet
    clippy::multiple_crate_versions,
    clippy::option_if_let_else,
)]

mod api;
mod cache;
mod cluster_admin;
mod database;
mod main_window;

type CosmicVergeClient = basws_client::Client<api::Client>;

use std::path::PathBuf;

use clap::Clap;
use tracing_subscriber::prelude::*;

#[macro_use]
extern crate tracing;

#[cfg(debug_assertions)]
const SERVER_URL: &str = "ws://localhost:7879/v1/ws";
#[cfg(not(debug_assertions))]
const SERVER_URL: &str = "wss://cosmicverge.com/v1/ws";

fn main() -> anyhow::Result<()> {
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

    let opt = CliOptions::parse();
    match opt.command.unwrap_or_default() {
        Command::Play {
            server_url,
            database,
        } => {
            database::Database::initialize(
                database.unwrap_or_else(|| PathBuf::from("cosmicverge.sled")),
            )?;
            main_window::run(server_url.as_deref().unwrap_or(SERVER_URL))
        }
        Command::Cluster => cluster_admin::run(),
    }
}

#[derive(Clap, Debug)]
#[clap(name = "cosmicverge", about = "The Cosmic Verge game client")]
struct CliOptions {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Clap, Debug)]
enum Command {
    /// Play Cosmic Verge
    Play {
        /// which server to connect to
        // In debug, default is ws://localhost:7879/v1/ws.
        // In release, default is wss://cosmicverge.com/v1/ws
        server_url: Option<String>,

        /// where to store the client data
        // default is `./cosmicverge.persy`
        // TODO in release mode, it should store it in the proper location in the user's home folder
        database: Option<PathBuf>,
    },
    /// Manage the Cluster
    // TODO this will need options that include a way to override the connection
    // URL. Need to figure out how remote access will work -- Jon doesn't want
    // to expose the cluster API without using some sort of VPN or tunnel
    Cluster,
}

impl Default for Command {
    fn default() -> Self {
        Self::Play {
            server_url: None,
            database: None,
        }
    }
}
