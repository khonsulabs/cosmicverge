use std::{
    fmt::{self, Debug, Formatter},
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use flume::r#async::RecvStream;
use futures_util::{stream::Stream, StreamExt};
use quinn::{CertificateChain, Endpoint, Incoming, NewConnection, ServerConfigBuilder};
use tokio::task::JoinHandle;

use crate::{
    certificate::{Certificate, PrivateKey},
    Connection, Error, Result,
};

/// TODO: docs
#[derive(Clone)]
pub struct Server {
    /// Initiate new connections or close socket.
    endpoint: Endpoint,
    /// Receiving new incoming connections.
    receiver: RecvStream<'static, Connection>,
    /// Task handle that handles new incoming connections.
    task: Arc<JoinHandle<Result<()>>>,
}

impl Debug for Server {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("endpoint", &self.endpoint)
            .field("receiver", &String::from("RecvStream<Connection>"))
            .field("task", &self.task)
            .finish()
    }
}

impl Server {
    /// TODO: improve docs
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one address
    /// - [`Error::Certificate`] if the [`Certificate`] couldn't be parsed
    /// - [`Error::PrivateKey`] if the [`PrivateKey`] couldn't be parsed
    /// - [`Error::InvalidKeyPair`] if failed to pair the given [`Certificate`] and [`PrivateKey`]
    /// - [`Error::BindSocket`] if the socket couldn't be bound to the given `address`
    #[allow(clippy::unwrap_in_result)]
    pub fn new<A: ToSocketAddrs>(
        address: A,
        //protocol: &str,
        certificate: &Certificate,
        private_key: &PrivateKey,
        //client_certs: impl ClientCerts + 'static,
        //filter: impl Filter + 'static,
    ) -> Result<Self> {
        let address = super::parse_socket(address)?;

        let certificate =
            quinn::Certificate::from_der(&certificate.0).map_err(Error::Certificate)?;
        let private_key = quinn::PrivateKey::from_der(&private_key.0).map_err(Error::PrivateKey)?;
        let chain = CertificateChain::from_certs(Some(certificate));

        let mut cfg_builder = ServerConfigBuilder::default();
        let _ = cfg_builder
            .certificate(chain, private_key)
            .map_err(Error::InvalidKeyPair)?
            /*.protocols(&[protocol.as_bytes()])*/;
        let mut cfg = cfg_builder.build();

        let transport = super::transport();
        cfg.transport = Arc::new(transport);

        // TODO: finish client certification
        // let tls_cfg = Arc::get_mut(&mut cfg.crypto).unwrap();
        // tls_cfg.set_client_certificate_verifier(Arc::new(ClientCertsWrapper(client_certs)));

        let mut endpoint_builder = Endpoint::builder();
        let _ = endpoint_builder.listen(cfg);
        let (endpoint, incoming) = endpoint_builder.bind(&address).map_err(Error::BindSocket)?;

        let (sender, receiver) = flume::unbounded();
        let receiver = receiver.into_stream();

        // TODO: configurable executor
        let task = Arc::new(tokio::spawn(Self::incoming(
            incoming, sender, /*, filter*/
        )));

        Ok(Self {
            endpoint,
            receiver,
            task,
        })
    }

    /// Handle incoming connections.
    /// TODO: improve docs
    async fn incoming(
        mut incoming: Incoming,
        sender: flume::Sender<Connection>, /*, filter: impl Filter + 'static */
    ) -> Result<()> {
        while let Some(connecting) = incoming.next().await {
            //let filter = filter.clone();
            let sender = sender.clone();

            /*let address = incoming.remote_address();
            let handshake = incoming
                .handshake_data()
                .await
                .map(Handshake::from)
                .map_err(Error::from);

            if filter
                .filter(address, handshake)
                .await
                .map_err(|e| e.downcast::<Error>().unwrap_or_else(Error::from))?
            {*/
            if let Ok(NewConnection {
                connection,
                bi_streams,
                ..
            }) = connecting.await
            {
                let connection = Connection {
                    connection,
                    bi_streams,
                };
                #[allow(clippy::expect_used)]
                sender
                    .send(connection)
                    .expect("no receiver for new incoming connections");
            } else {
                // TODO: add logging point
            }
            //}
        }

        Ok(())
    }

    /// TODO: improve docs
    ///
    /// # Errors
    /// [`Error::LocalAddress`] if aquiring the local address failed.
    pub fn local_address(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(Error::LocalAddress)
    }
}

impl Stream for Server {
    type Item = Connection;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}

/*#[async_trait]
pub trait ClientCerts: Send + Sync {
    async fn verify(&self, certificate: Certificate) -> Result<(), webpki::Error>;
}

#[async_trait]
impl<C: Fn(Certificate) -> Result<(), webpki::Error> + Send + Sync + 'static> ClientCerts for C {
    async fn verify(&self, certificate: Certificate) -> Result<(), webpki::Error> {
        self(certificate)
    }
}

struct ClientCertsWrapper<T: ClientCerts>(T);

impl<T: ClientCerts> ClientCertVerifier for ClientCertsWrapper<T> {
    // TODO: allow CA authorized client certificates by using `webpki-roots`
    fn client_auth_root_subjects(&self, _sni: Option<&DNSName>) -> Option<DistinguishedNames> {
        Some(Vec::new())
    }

    fn verify_client_cert(
        &self,
        presented_certs: &[rustls::Certificate],
        _sni: Option<&DNSName>,
    ) -> Result<ClientCertVerified, TLSError> {
        if let Some(cert) = presented_certs.get(0).map(|c| &c.0) {
            // TODO: replace with our own executor
            futures_executor::block_on(self.0.verify(Certificate(cert.clone())))
                .map(|_| ClientCertVerified::assertion())
                .map_err(TLSError::WebPKIError)
        } else {
            Err(TLSError::NoCertificatesPresented)
        }
    }

    fn client_auth_mandatory(&self, _sni: Option<&DNSName>) -> Option<bool> {
        Some(false)
    }
}

#[async_trait]
pub trait Filter: Clone + Send + Sync {
    async fn filter(
        &self,
        address: SocketAddr,
        handshake: Result<Handshake>,
    ) -> anyhow::Result<bool>;
}

#[async_trait]
impl<
        F: Fn(SocketAddr, Result<Handshake>) -> anyhow::Result<bool> + Clone + Send + Sync + 'static,
    > Filter for F
{
    async fn filter(
        &self,
        address: SocketAddr,
        handshake: Result<Handshake>,
    ) -> anyhow::Result<bool> {
        self(address, handshake)
    }
}

pub struct Handshake {
    protocol: Result<String, Error>,
    server_name: Option<String>,
}

impl From<HandshakeData> for Handshake {
    fn from(
        HandshakeData {
            protocol,
            server_name,
        }: HandshakeData,
    ) -> Self {
        let protocol = protocol.ok_or(Error::NoProtocol).and_then(|p| {
            String::from_utf8(p).map_err(|error| Error::ParseProtocol {
                protocol: String::from_utf8_lossy(error.as_bytes()).into_owned(),
                error,
            })
        });

        Self {
            protocol,
            server_name,
        }
    }
}

#[test]
fn blubb() {
    Server::new(
        ([1, 1, 1, 1], 84),
        "s",
        &Certificate(vec![]),
        &PrivateKey(vec![]),
        |_| todo!(),
        move |_, _| {
            print!("{}", blubb[0]);
            print!("{:?}", gaga);
            Ok(true)
        },
    )
    .unwrap();
}*/
