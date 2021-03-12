mod connection;
#[allow(dead_code)] // this mod exposes functions in lib.rs, but appears unused in the exe
mod migrations;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    connection::initialize(None).await;
    migrations::run_all().await.unwrap();
}
