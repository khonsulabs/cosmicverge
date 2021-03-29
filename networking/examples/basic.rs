use anyhow::{Error, Result};
use cosmicverge_networking as networking;
use futures_util::StreamExt;
use networking::{Client, Server};

#[tokio::main]
async fn main() -> Result<()> {
    let (certificate, private_key) = networking::generate_self_signed("test");

    let server = {
        let certificate = certificate.clone();

        tokio::spawn(async move {
            let mut server = Server::new("[::1]:5000", &certificate, &private_key)?;
            println!("[server] Listening on {}", server.local_address()?);

            let mut connections = Vec::new();

            while let Some(mut connection) = server.next().await {
                println!("[server] New Connection: {}", connection.remote_address());

                connections.push(tokio::spawn(async move {
                    while let Some((sender, receiver)) = connection.next().await.transpose()? {
                        println!(
                            "[server] New incoming stream from: {}",
                            connection.remote_address()
                        );
                    }

                    Result::<_, Error>::Ok(())
                }));
            }

            Result::<_, Error>::Ok(())
        })
    };

    let client = tokio::spawn(async move {
        let client = Client::new("[::1]:5001", &certificate)?;
        let connection = client.connect("[::1]:5000", "test").await?;
        println!(
            "[client] Successfully connected to {}",
            connection.remote_address()
        );

        let (send, receiver) = connection.open_stream().await?;
        println!(
            "[client] Successfully opened stream to {}",
            connection.remote_address()
        );

        Result::<_, Error>::Ok(())
    });

    server.await??;
    client.await??;

    Ok(())
}
