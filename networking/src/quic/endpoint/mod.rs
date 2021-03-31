//! Starting point to create a QUIC enabled network socket.

mod transport;

use std::{
    fmt::{self, Debug, Formatter},
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use flume::{r#async::RecvStream, Sender};
use futures_channel::oneshot;
use futures_util::{stream::Stream, StreamExt};
use quinn::{
    CertificateChain, ClientConfig, ClientConfigBuilder, NewConnection, ServerConfig,
    ServerConfigBuilder, VarInt,
};

use super::{StreamExtExt, Task};
use crate::{
    certificate::{Certificate, PrivateKey},
    Connection, Error, Result,
};

/// Represents a socket using the QUIC protocol to communicate with peers.
/// Receives incoming [`Connection`]s through [`Stream`].
#[derive(Clone)]
pub struct Endpoint {
    /// Initiate new connections or close socket.
    endpoint: quinn::Endpoint,
    /// Receiving new incoming connections.
    receiver: RecvStream<'static, Result<Connection>>,
    /// Task handle handling new incoming connections.
    task: Task<()>,
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("endpoint", &self.endpoint)
            .field("receiver", &String::from("RecvStream<Connection>"))
            .field("task", &self.task)
            .finish()
    }
}

impl Endpoint {
    /// Encapsulates common construction paths for
    /// [`new_server`](Endpoint::new_server) and
    /// [`new_client`](Endpoint::new_client).
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one
    ///   address
    /// - [`Error::BindSocket`] if the socket couldn't be bound to the given
    ///   `address`
    fn new(
        address: impl ToSocketAddrs,
        server_config: ServerConfig,
        client_config: ClientConfig,
    ) -> Result<Self> {
        // parse socket, this can return two different errors, see documentation
        let address = parse_socket(address)?;

        // configure endpoint for server and client
        let mut endpoint_builder = quinn::Endpoint::builder();
        let _ = endpoint_builder
            .listen(server_config)
            .default_client_config(client_config);

        // build endpoint
        let (endpoint, incoming) = endpoint_builder
            .bind(&address)
            .map_err(|error| Error::BindSocket(Arc::new(error)))?;

        // create channels that will receive incoming `Connection`s
        let (sender, receiver) = flume::unbounded();
        let receiver = receiver.into_stream();

        // spawn task handling incoming `Connection`s
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let task = Task::new(
            Self::incoming(incoming, sender, shutdown_receiver),
            shutdown_sender,
        );

        Ok(Self {
            endpoint,
            receiver,
            task,
        })
    }

    /// Simplified version of creating a server.
    /// TODO: link builder
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one
    ///   address
    /// - [`Error::Certificate`] if the [`Certificate`] couldn't be parsed
    /// - [`Error::PrivateKey`] if the [`PrivateKey`] couldn't be parsed
    /// - [`Error::InvalidKeyPair`] if failed to pair the given [`Certificate`]
    ///   and [`PrivateKey`]
    /// - [`Error::BindSocket`] if the socket couldn't be bound to the given
    ///   `address`
    pub fn new_server<A: ToSocketAddrs>(
        address: A,
        certificate: &Certificate,
        private_key: &PrivateKey,
    ) -> Result<Self> {
        let mut server_config_builder = ServerConfigBuilder::default();

        // parse certificates
        let certificate =
            quinn::Certificate::from_der(&certificate.0).map_err(Error::Certificate)?;
        let private_key = quinn::PrivateKey::from_der(&private_key.0).map_err(Error::PrivateKey)?;
        let chain = CertificateChain::from_certs(Some(certificate));

        // configure server
        let _ = server_config_builder
            .certificate(chain, private_key)
            .map_err(Error::InvalidKeyPair)?;
        let mut server_config = server_config_builder.build();

        // configure transport settings for server
        let transport = transport::transport();
        server_config.transport = Arc::new(transport);

        // build endpoint
        Self::new(address, server_config, ClientConfig::default())
    }

    /// Simplified version of creating a client.
    /// TODO: link builder
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one
    ///   address
    /// - [`Error::Certificate`] if the [`Certificate`] couldn't be parsed
    /// - [`Error::InvalidCertificate`] if the [`Certificate`] couldn't be added
    ///   as a certificate authority
    /// - [`Error::BindSocket`] if the socket couldn't be bound to the given
    ///   `address`
    pub fn new_client<A: ToSocketAddrs>(address: A, certificate: &Certificate) -> Result<Self> {
        let mut client_config_builder = ClientConfigBuilder::default();

        // parse certificates
        let certificate =
            quinn::Certificate::from_der(&certificate.0).map_err(Error::Certificate)?;

        // configure client
        let _ = client_config_builder
            .add_certificate_authority(certificate)
            .map_err(Error::InvalidCertificate)?;
        let mut client_config = client_config_builder.build();

        // configure transport settings for client
        let transport = transport::transport();
        client_config.transport = Arc::new(transport);

        // build endpoint
        Self::new(address, ServerConfig::default(), client_config)
    }

    /// Handle incoming connections. Accessed through [`Stream`] of
    /// [`Endpoint`].
    async fn incoming(
        incoming: quinn::Incoming,
        sender: Sender<Result<Connection>>,
        mut shutdown: oneshot::Receiver<()>,
    ) {
        let mut incoming = incoming.fuse_last();

        // TODO: fix clippy
        #[allow(clippy::mut_mut, clippy::panic)]
        while let Some(connecting) = futures_util::select_biased! {
            connecting = incoming.select_next_some() => connecting,
            _ = shutdown => None,
            complete => unreachable!("stream should have ended when `incoming` returned `None`"),
        } {
            let connection = connecting
                .await
                .map(
                    |NewConnection {
                         connection,
                         bi_streams,
                         ..
                     }| Connection::new(connection, bi_streams),
                )
                .map_err(Error::IncomingConnection);

            // if there is no receiver, it means that we dropped the last `Endpoint`
            if sender.send(connection).is_err() {
                break;
            }
        }
    }

    /// Establish a new [`Connection`] to a client. The `server_name` validates
    /// the certificate.
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one
    ///   address
    /// - [`Error::Connect`] if no connection to the given `address` could be
    ///   established
    /// - [`Error::Connecting`] if the connection to the given `address` failed
    pub async fn connect<A: ToSocketAddrs>(
        &self,
        address: A,
        server_name: &str,
    ) -> Result<Connection> {
        let address = parse_socket(address)?;

        let connecting = self
            .endpoint
            .connect(&address, server_name)
            .map_err(Error::Connect)?;

        let NewConnection {
            connection,
            bi_streams,
            ..
        } = connecting.await.map_err(Error::Connecting)?;

        Ok(Connection::new(connection, bi_streams))
    }

    /// Get the local [`SocketAddr`] the underlying socket is bound to.
    ///
    /// # Errors
    /// [`Error::LocalAddress`] if aquiring the local address failed.
    pub fn local_address(&self) -> Result<SocketAddr> {
        self.endpoint
            .local_addr()
            .map_err(|error| Error::LocalAddress(Arc::new(error)))
    }

    /// Prevents any new incoming connections. Already incoming connections will
    /// finish first.
    ///
    /// # Errors
    /// [`Error::AlreadyClosed`] if it was already closed.
    pub async fn close_incoming(&self) -> Result<()> {
        self.task.close(()).await
    }

    /// Wait for all [`Connection`]s to the [`Endpoint`] to be cleanly shut
    /// down. Does not close existing connections or cause incoming
    /// connections to be rejected. See
    /// [`close_incoming`](`Self::close_incoming`).
    pub async fn wait_idle(&self) {
        self.endpoint.wait_idle().await;
    }

    /// Close all of this [`Endpoint`]'s [`Connection`]s immediately and cease
    /// accepting new [`Connection`]s.
    ///
    /// To close an [`Endpoint`] gracefully use
    /// [`close_incoming`](Self::close_incoming),
    /// [`Sender::finish`](crate::Sender::finish) and
    /// [`wait_idle`](Self::wait_idle).
    ///
    /// # Errors
    /// [`Error::AlreadyClosed`] if it was already closed.
    pub async fn close(&self) -> Result<()> {
        self.endpoint.close(VarInt::from_u32(0), &[]);
        self.task.abort().await
    }
}

impl Stream for Endpoint {
    type Item = Result<Connection>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}

/// Parses [`ToSocketAddrs`] and additionally generates an error if more then
/// one address was passed.
///
/// # Errors
/// - [`Error::ParseAddress`] if the `address` couldn't be parsed
/// - [`Error::MultipleAddresses`] if the `address` contained more then one
///   address
fn parse_socket(address: impl ToSocketAddrs) -> Result<SocketAddr> {
    let mut addresses = address
        .to_socket_addrs()
        .map_err(|error| Error::ParseAddress(Arc::new(error)))?;
    #[allow(clippy::expect_used)]
    let address = addresses
        .next()
        .expect("`ToSocketAddrs` should always have at least one address");

    if addresses.next().is_some() {
        Err(Error::MultipleAddresses)
    } else {
        Ok(address)
    }
}
