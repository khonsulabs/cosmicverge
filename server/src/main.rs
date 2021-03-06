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

use cli::accounts::AccountCommand;
use structopt::StructOpt;

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
struct Cli {
    /// The command to execute
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(about = "commands to execute")]
/// commands that the server can execute
enum Command {
    /// Run the Server
    Serve,
    /// Generate static assets, currently just includes procedurally generated planets
    GenerateAssets {
        /// The path to the static folder to generate the assets within
        static_folder: PathBuf,
    },
    Account {
        /// The ID of the account
        #[structopt(long)]
        id: Option<i64>,

        /// The Twitch handle to look up to find the account
        #[structopt(long)]
        twitch: Option<String>,

        /// The command to execute
        #[structopt(subcommand)]
        command: AccountCommand,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Error initializing environment");
    initialize_logging();

    let cli = Cli::from_args();
    match cli.command {
        Command::Serve => http::run_webserver().await,
        Command::GenerateAssets { static_folder } => generate_assets(static_folder).await,
        Command::Account {
            id,
            twitch,
            command,
        } => cli::accounts::handle_command(id, twitch, command).await,
    }
}

pub fn initialize_logging() {
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .try_init()
        .unwrap();
}

async fn generate_assets(static_folder: PathBuf) -> anyhow::Result<()> {
    planets::generate_assets(&static_folder);
    Ok(())
}
