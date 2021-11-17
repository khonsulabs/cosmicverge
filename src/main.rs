use std::path::Path;

use async_trait::async_trait;
use axum::{error_handling::HandleErrorExt, routing::service_method_routing::get, Router};
use bonsaidb::server::{
    AcmeConfiguration, Configuration, DefaultPermissions, Peer, Server, StandardTcpProtocols,
    TcpService,
};
use hyper::{server::conn::Http, StatusCode};
use tower_http::services::ServeDir;

#[derive(Debug, Clone)]
pub struct Website;

#[async_trait]
impl TcpService for Website {
    type ApplicationProtocols = StandardTcpProtocols;

    async fn handle_connection<
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    >(
        &self,
        connection: S,
        peer: &Peer<Self::ApplicationProtocols>,
    ) -> Result<(), S> {
        let app = Router::new().nest(
            "/",
            get(ServeDir::new("static")).handle_error(|err: std::io::Error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("unhandled internal error: {}", err),
                )
            }),
        );

        if let Err(err) = Http::new().serve_connection(connection, app).await {
            log::error!("[http] error serving {}: {:?}", peer.address, err);
        }

        Ok(())
    }
}

#[cfg(debug_assertions)]
const HTTP_LISTEN: &str = ":::8080";

#[cfg(not(debug_assertions))]
const HTTP_LISTEN: &str = ":::80";
#[cfg(not(debug_assertions))]
const HTTPS_LISTEN: &str = ":::443";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let server = Server::open(
        Path::new("cosmicverge.bonsaidb"),
        Configuration {
            server_name: String::from("cosmicverge.com"),
            default_permissions: DefaultPermissions::AllowAll,
            acme: AcmeConfiguration {
                contact_email: Some(String::from("mailto:netops@khonsulabs.com")),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .await?;

    let task_server = server.clone();
    tokio::spawn(async move { task_server.listen_for_tcp_on(HTTP_LISTEN, Website).await });
    #[cfg(not(debug_assertions))]
    {
        let task_server = server.clone();
        tokio::spawn(async move {
            task_server
                .listen_for_secure_tcp_on(HTTPS_LISTEN, Website)
                .await
        });
    }

    server.listen_for_shutdown().await?;

    Ok(())
}
