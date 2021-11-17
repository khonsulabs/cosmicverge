//! Shows how to use the axum web framework with BonsaiDb. Any hyper-compatible
//! framework should be usable.

use std::path::Path;

use async_trait::async_trait;
use axum::{error_handling::HandleErrorExt, routing::service_method_routing::get, Router};
use bonsaidb::server::{
    AcmeConfiguration, Backend, Configuration, CustomServer, DefaultPermissions, NoDispatcher,
};
use hyper::{server::conn::Http, StatusCode};
use tower_http::services::ServeDir;

/// The `AxumBackend` implements `Backend` and overrides
/// `handle_http_connection` by serving the response using
/// [`axum`](https://github.com/tokio-rs/axum).
#[derive(Debug)]
pub struct AxumBackend;

#[async_trait]
impl Backend for AxumBackend {
    async fn handle_http_connection<
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    >(
        connection: S,
        peer_address: std::net::SocketAddr,
        _server: &CustomServer<Self>,
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
            log::error!("[http] error serving {}: {:?}", peer_address, err);
        }

        Ok(())
    }

    type CustomApi = ();
    type CustomApiDispatcher = NoDispatcher<Self>;
    type ClientData = ();
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
    let server = CustomServer::<AxumBackend>::open(
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
    tokio::spawn(async move { task_server.listen_for_http_on(HTTP_LISTEN).await });
    #[cfg(not(debug_assertions))]
    {
        let task_server = server.clone();
        tokio::spawn(async move { task_server.listen_for_https_on(HTTPS_LISTEN).await });
    }

    server.listen_for_shutdown().await?;

    Ok(())
}
