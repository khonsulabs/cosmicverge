mod connection;

// cargo warns about code like `migrations::undo_all` not being used when
// compiling this binary. The warning is incorrect, however, because this module
// is public when consuming this as a lib. We could mark this mod as pub, but it
// "feels" more correct to ignore the warning in this binary.
#[allow(dead_code)]
mod migrations;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    connection::initialize(None).await;
    migrations::run_all().await.unwrap();
}
