mod connection;
mod migrations;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    connection::initialize().await;
    migrations::run_all().await.unwrap();
}
