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
struct CLI {
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

    let cli = CLI::from_args();
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
    planets::generate_assets(static_folder);
    Ok(())
}
