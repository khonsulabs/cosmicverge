#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use structopt::StructOpt;

/// the definition of the http server.
mod http;
/// parsing support for JSON Web Keys
mod jwk;
/// controls the game loop logic
mod orchestrator;
/// defines procedurally generated planets
mod planets;
/// pub-sub message exchanging with other servers
mod pubsub;
/// shared connection pools
mod redis;
/// a helper type making it easier to read locking code
mod redis_lock;
/// the websocket api logic
mod server;
/// twitch oauth support
mod twitch;

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
    Serve,
    GenerateAssets { static_folder: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Error initializing environment");
    initialize_logging();

    let cli = CLI::from_args();
    match cli.command {
        Command::Serve => http::run_webserver().await,
        Command::GenerateAssets { static_folder } => generate_assets(static_folder).await,
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
