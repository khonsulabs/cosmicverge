use std::io;

use quinn::{ConnectError, ConnectionError, EndpointError, ParseError};
use rustls::TLSError;
use thiserror::Error;

/// [`Result`](std::result::Result) type for this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// TODO: docs
#[derive(Debug, Error)]
pub enum Error {
    /// Returned by [`Server::new`](crate::Server::new)/[`Client::new`](crate::Client::new) when failing to parse the given `address`.
    #[error("Failed parsing address: {0}")]
    ParseAddress(io::Error),
    /// Returned by [`Server::new`](crate::Server::new)/[`Client::new`](crate::Client::new) when given more then one address.
    #[error("Multiple addresses are not supported")]
    MultipleAddresses,
    /// Returned by [`Server::new`](crate::Server::new)/[`Client::new`](crate::Client::new) when failing to parse the given [`Certificate`](crate::Certificate).
    #[error("Failed parsing certificate: {0}")]
    Certificate(ParseError),
    /// Returned by [`Server::new`](crate::Server::new) when failing to parse the given [`PrivateKey`](crate::PrivateKey).
    #[error("Failed parsing private key: {0}")]
    PrivateKey(ParseError),
    /// Returned by [`Server::new`](crate::Server::new) when failing to pair the given [`Certificate`](crate::Certificate) and [`PrivateKey`](crate::PrivateKey).
    #[error("Invalid certificate key pair: {0}")]
    InvalidKeyPair(TLSError),
    /// Returned by [`Client::new`](crate::Client::new) when failing to add the given [`Certificate`](crate::Certificate) as a certificate authority.
    #[error("Invalid certificate: {0}")]
    InvalidCertificate(webpki::Error),
    /// Returned by [`Server::new`](crate::Server::new)/[`Client::new`](crate::Client::new) when failing to bind the socket on the given `address`.
    #[error("Failed to bind socket: {0}")]
    BindSocket(EndpointError),
    /// Returned by [`Server::local_address`](crate::Server::local_address)/[`Client::local_address`](crate::Client::local_address) when failing to aquire the local address.
    #[error("Failed to aquire local address: {0}")]
    LocalAddress(io::Error),
    /// Returned by [`Connection`](crate::Connection) [`Stream`](futures_util::stream::Stream) when receiving a new stream failed.
    #[error("Error on receiving a new stream: {0}")]
    IncomingStream(ConnectionError),
    /// Returned by [`Client::connect`](crate::Client::connect) if establishing a connection to the given `address` failed.
    #[error("Error on establishing a connection to a remote address: {0}")]
    Connect(ConnectError),
    /// Returned by [`Client::connect`](crate::Client::connect) if connecting to the remote `address` failed.
    #[error("Error on connecting to a remote address: {0}")]
    Connecting(ConnectionError),
    /// Returned by [`Connection::open_stream`](crate::Connection::open_stream) if opening a stream failed.
    #[error("Error on opening a stream: {0}")]
    OpenStream(ConnectionError),
    /*#[error("no protocol")]
    NoProtocol,
    #[error("parsing protocol: {protocol}")]
    ParseProtocol {
        protocol: String,
        error: FromUtf8Error,
    },*/
}
