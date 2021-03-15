mod connection;
mod migrations;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    connection::initialize(None).await;
    migrations::run_all().await.unwrap();
}
