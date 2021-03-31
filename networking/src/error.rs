//! [`Error`](std::error::Error) for this [`crate`].

pub use std::io::Error as IoError;
use std::sync::Arc;

pub use bincode::ErrorKind;
pub use quinn::{ConnectError, ConnectionError, EndpointError, ParseError, ReadError, WriteError};
pub use rustls::TLSError;
use thiserror::Error;

/// [`Result`](std::result::Result) type for this [`crate`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// [`Error`](std::error::Error) for this [`crate`].
#[derive(Clone, Debug, Error)]
pub enum Error {
    /// Failed to parse the given address.
    #[error("Failed parsing address: {0}")]
    ParseAddress(Arc<IoError>),
    /// Multiple addresses are not supported.
    #[error("Multiple addresses are not supported")]
    MultipleAddresses,
    /// Returned by [`Endpoint`](crate::Endpoint) when failing to parse the
    /// given [`Certificate`](crate::Certificate).
    #[error("Failed parsing certificate: {0}")]
    Certificate(ParseError),
    /// Returned by [`Endpoint`](crate::Endpoint) when failing to parse the
    /// given [`PrivateKey`](crate::PrivateKey).
    #[error("Failed parsing private key: {0}")]
    PrivateKey(ParseError),
    /// Returned by [`Endpoint`](crate::Endpoint) when failing to pair the given
    /// [`Certificate`](crate::Certificate) and
    /// [`PrivateKey`](crate::PrivateKey).
    #[error("Invalid certificate key pair: {0}")]
    InvalidKeyPair(TLSError),
    /// Returned by [`Endpoint`](crate::Endpoint) when failing to add the given
    /// [`Certificate`](crate::Certificate) as a certificate authority.
    #[error("Invalid certificate: {0}")]
    InvalidCertificate(webpki::Error),
    /// Returned by [`Endpoint`](crate::Endpoint) when failing to bind the
    /// socket on the given `address`.
    #[error("Failed to bind socket: {0}")]
    BindSocket(Arc<EndpointError>),
    /// Returned by [`Endpoint`](crate::Endpoint)
    /// [`Stream`](futures_util::stream::Stream) when receiving a new stream
    /// failed.
    #[error("Error on receiving a new connection: {0}")]
    IncomingConnection(ConnectionError),
    /// Returned by [`Endpoint::local_address`](crate::Endpoint::local_address)
    /// when failing to aquire the local address.
    #[error("Failed to aquire local address: {0}")]
    LocalAddress(Arc<IoError>),
    /// Attempting to close something that is already closed.
    #[error("This is already closed")]
    AlreadyClosed,
    /// Returned by [`Endpoint::connect`](crate::Endpoint::connect) if
    /// establishing a connection to the given `address` failed.
    #[error("Error on establishing a connection to a remote address: {0}")]
    Connect(ConnectError),
    /// Returned by [`Endpoint::connect`](crate::Endpoint::connect) if
    /// connecting to the remote `address` failed.
    #[error("Error on connecting to a remote address: {0}")]
    Connecting(ConnectionError),
    /// Returned by [`Connection`](crate::Connection)
    /// [`Stream`](futures_util::stream::Stream) when receiving a new stream
    /// failed.
    #[error("Error on receiving a new stream: {0}")]
    ReceiveStream(ConnectionError),
    /// Returned by [`Connection::open_stream`](crate::Connection::open_stream)
    /// if opening a stream failed.
    #[error("Error on opening a stream: {0}")]
    OpenStream(ConnectionError),
    /// Returned by [`Sender::finish`](crate::Sender::finish) if
    /// [`Sender`](crate::Sender) failed to write into the stream.
    #[error("Error writing to a stream: {0}")]
    Write(WriteError),
    /// Returned by [`Sender::finish`](crate::Sender::finish) if
    /// [`Sender`](crate::Sender) failed to finish a stream.
    #[error("Error finishing a stream: {0}")]
    Finish(WriteError),
    /// Returned by [`Sender::send`](crate::Sender::send) if the stream was
    /// closed by [`Sender::finish`](crate::Sender::finish) or the
    /// [`Connection`](crate::Connection) or [`Endpoint`](crate::Endpoint) was
    /// closed or dropped.
    #[error("Stream was closed")]
    Send,
    /// Returned by [`Sender::send`](crate::Sender::send) if
    /// [`serialization`](serde::Serialize) failed.
    #[error("Error serializing to a stream: {0}")]
    Serialize(Arc<ErrorKind>),
    /// Returned by [`Receiver::close`](crate::Receiver::close) if
    /// [`Receiver`](crate::Receiver) failed to read from a stream.
    #[error("Error reading from a stream: {0}")]
    Read(ReadError),
    /// Returned by [`Receiver::finish`](crate::Receiver::finish) if
    /// [`Receiver`](crate::Receiver) failed to
    /// [`deserialize`](serde::Deserialize) from a stream.
    #[error("Error deserializing from a stream: {0}")]
    Deserialize(Arc<ErrorKind>),
}
