use quinn::{EndpointError, ParseError};
use rustls::TLSError;
use thiserror::Error;

/// [`Result`](std::result::Result) type for this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// TODO: docs
#[derive(Debug, Error)]
pub enum Error {
    /// Returned by [`Server::new`](crate::Server::new) when failing to parse the given [`Certificate`](crate::Certificate).
    #[error("Failed parsing certificate: {0}")]
    Certificate(ParseError),
    /// Returned by [`Server::new`](crate::Server::new) when failing to parse the given [`PrivateKey`](crate::PrivateKey).
    #[error("Failed parsing private key: {0}")]
    PrivateKey(ParseError),
    /// Returned by [`Server::new`](crate::Server::new) when failing to pair the given [`Certificate`](crate::Certificate) and [`PrivateKey`](crate::PrivateKey).
    #[error("Found invalid certificate key pair: {0}")]
    InvalidKeyPair(TLSError),
    /// Returned by [`Server::new`](crate::Server::new) when failing to bind the socket on the givven address.
    #[error("Failed to bind socket: {0}")]
    BindSocket(EndpointError),
    /*#[error("no protocol")]
    NoProtocol,
    #[error("parsing protocol: {protocol}")]
    ParseProtocol {
        protocol: String,
        error: FromUtf8Error,
    },*/
}
