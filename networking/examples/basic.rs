use anyhow::{Error, Result};
use cosmicverge_networking as networking;
use futures_util::StreamExt;
use networking::{Client, Server};

const SOCKET: &str = "[::1]";
const SERVER_NAME: &str = "test";
const SERVER_PORT: &str = "5000";
const CLIENTS: usize = 100;

#[tokio::main]
async fn main() -> Result<()> {
    // generate a certificate pair
    let (certificate, private_key) = networking::generate_self_signed(SERVER_NAME);

    // start the server
    let server = {
        let certificate = certificate.clone();

        // build the server
        // we want to do this outside to reserve the `SERVER_PORT`, otherwise spawned
        // clients may take it
        let mut server = Server::new(
            format!("{}:{}", SOCKET, SERVER_PORT),
            &certificate,
            &private_key,
        )?;

        tokio::spawn(async move {
            println!("[server] Listening on {}", server.local_address()?);

            // collect incoming connection tasks
            let mut connections = Vec::with_capacity(CLIENTS);

            // start listening to new incoming connections
            // in this example we know there is `CLIENTS` number of clients, so we will not
            // wait for more
            for _ in 0..CLIENTS {
                let mut connection = server.next().await.expect("connection failed");
                println!("[server] New Connection: {}", connection.remote_address());

                // every new incoming connections is handled in it's own task
                connections.push(tokio::spawn(async move {
                    // start listening to new incoming streams
                    // in this example we know there is only 1 incoming stream, so we will not wait
                    // for more
                    let (mut sender, receiver) =
                        connection.next().await.expect("no stream found")?;
                    println!(
                        "[server] New incoming stream from: {}",
                        connection.remote_address()
                    );

                    // wait for stream to finish
                    sender.finish().await?;

                    Result::<_, Error>::Ok(())
                }));
            }

            // wait for all connections to finish
            for connection in connections {
                connection.await??;
            }

            // wait for server to finish cleanly
            server.wait_idle().await;
            println!("[server] Successfully finished {}", server.local_address()?);

            Result::<_, Error>::Ok(())
        })
    };

    // collect client tasks
    let mut clients = Vec::with_capacity(CLIENTS);

    // start 100 clients
    for index in 0..CLIENTS {
        let certificate = certificate.clone();

        clients.push(tokio::spawn(async move {
            // build a client
            let client = Client::new(format!("{}:0", SOCKET), &certificate)?;
            println!("[client:{}] Bound to {}", index, client.local_address()?);

            let connection = client
                .connect(format!("{}:{}", SOCKET, SERVER_PORT), SERVER_NAME)
                .await?;
            println!(
                "[client:{}] Successfully connected to {}",
                index,
                connection.remote_address()
            );

            // initiate a stream
            let (mut sender, receiver) = connection.open_stream().await?;
            println!(
                "[client:{}] Successfully opened stream to {}",
                index,
                connection.remote_address()
            );

            sender.finish().await?;

            // wait for client to finish cleanly
            client.wait_idle().await;
            println!(
                "[client:{}] Successfully finished {}",
                index,
                client.local_address()?
            );

            Result::<_, Error>::Ok(())
        }));
    }

    server.await??;

    for client in clients {
        client.await??;
    }

    Ok(())
}
