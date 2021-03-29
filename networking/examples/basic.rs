use anyhow::{Error, Result};
use cosmicverge_networking as networking;
use futures_util::StreamExt;
use networking::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let (certificate, private_key) = networking::generate_self_signed("test");

    let server = tokio::spawn(async move {
        let mut server = Server::new("[::1]:5000", &certificate, &private_key)?;

        while let Some(connection) = server.next().await {
            println!("New Connection: {}", connection.remote_address());
        }

        Result::<_, Error>::Ok(())
    });

    server.await??;

    Ok(())
}
