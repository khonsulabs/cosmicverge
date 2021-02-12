#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use structopt::StructOpt;

mod http;
mod jwk;
mod orchestrator;
mod planets;
mod pubsub;
mod redis;
mod redis_lock;
mod server;
mod twitch;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "cosmicverge",
    about = "Cosmic Verge web server and associated tools"
)]
struct CLI {
    /// The command to execute
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(about = "commands to execute")]
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

fn env(var: &str) -> String {
    std::env::var(var).unwrap()
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
