use std::path::Path;

use async_trait::async_trait;
use axum::{
    error_handling::HandleErrorExt, http::HeaderValue, routing::service_method_routing::get, Router,
};
use bonsaidb::server::{
    AcmeConfiguration, Configuration, DefaultPermissions, HttpService, Peer, Server,
    StandardTcpProtocols,
};
use hyper::{header, server::conn::Http, Body, StatusCode};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Debug, Clone)]
pub struct Website;

#[async_trait]
impl HttpService for Website {
    async fn handle_connection<
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    >(
        &self,
        connection: S,
        peer: &Peer<StandardTcpProtocols>,
    ) -> Result<(), S> {
        let app = router(peer.secure);
        // Add HSTS header.
        let app = app.layer(SetResponseHeaderLayer::<_, Body>::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; preload"),
        ));

        if let Err(err) = Http::new().serve_connection(connection, app).await {
            log::error!("[http] error serving {}: {:?}", peer.address, err);
        }

        Ok(())
    }
}

fn website() -> Router {
    Router::new().nest(
        "/",
        get(ServeDir::new("static")).handle_error(|err: std::io::Error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("unhandled internal error: {}", err),
            )
        }),
    )
}

#[cfg(debug_assertions)]
fn router(_secure: bool) -> Router {
    website()
}

#[cfg(not(debug_assertions))]
fn router(secure: bool) -> Router {
    if secure {
        website()
    } else {
        Router::new().nest("/", axum::routing::get(redirect_to_https))
    }
}

#[cfg(not(debug_assertions))]
async fn redirect_to_https(req: hyper::Request<Body>) -> hyper::Response<Body> {
    let path = req.uri().path();
    let mut response = hyper::Response::new(Body::empty());
    *response.status_mut() = StatusCode::PERMANENT_REDIRECT;
    response.headers_mut().insert(
        "Location",
        HeaderValue::from_str(&format!("https://cosmicverge.com{}", path)).unwrap(),
    );
    response
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
