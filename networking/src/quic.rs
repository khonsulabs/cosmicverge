use std::{net::SocketAddr, sync::Arc};

use flume::{Receiver, Sender};
use futures_util::StreamExt;
use quinn::{
    CertificateChain, Endpoint, Incoming, IncomingBiStreams, NewConnection, ServerConfigBuilder,
    TransportConfig,
};
use tokio::task::JoinHandle;

use crate::{
    certificate::{Certificate, PrivateKey},
    Error, Result,
};

/// TODO: docs
#[derive(Clone, Debug)]
pub struct Server {
    /// Initiate new connections or close socket.
    endpoint: Endpoint,
    /// Receiving new incoming connections.
    receiver: Receiver<Connection>,
    /// Task handle that handles new incoming connections.
    task: Arc<JoinHandle<Result<()>>>,
}

impl Server {
    /// TODO: improve docs
    ///
    /// # Errors
    /// - [`Error::Certificate`] if the certificate couldn't be parsed
    /// - [`Error::PrivateKey`] if the private key couldn't be parsed
    #[allow(clippy::unwrap_in_result)]
    pub fn new(
        address: impl Into<SocketAddr>,
        protocol: &str,
        certificate: &Certificate,
        private_key: &PrivateKey,
        //client_certs: impl ClientCerts + 'static,
        //filter: impl Filter + 'static,
    ) -> Result<Self> {
        let certificate =
            quinn::Certificate::from_der(&certificate.0).map_err(Error::Certificate)?;
        let private_key = quinn::PrivateKey::from_der(&private_key.0).map_err(Error::PrivateKey)?;
        let chain = CertificateChain::from_certs(Some(certificate));

        let mut cfg_builder = ServerConfigBuilder::default();
        let _ = cfg_builder
            .certificate(chain, private_key)
            .map_err(Error::InvalidKeyPair)?
            .protocols(&[protocol.as_bytes()]);
        let mut cfg = cfg_builder.build();

        let mut transport = TransportConfig::default();
        #[allow(clippy::expect_used)]
        let _ = transport
            // TODO: research if this is necessary, it improves privacy, but may hurt network providers?
            .allow_spin(false)
            // TODO: we are assuming that credit handling per connection will prevent crypto buffer from going out of bounds
            .crypto_buffer_size(usize::MAX)
            // TODO: handle keep-alive and time-out
            // transport.keep_alive_interval(); // heartbeat to prevent time-out, only needs to be sent from one side
            // transport.max_idle_timeout(); // time before being dropped
            // this API has no support for sending unordered data
            .datagram_receive_buffer_size(None)
            // TODO: support more then a single bidi-stream per connection
            .max_concurrent_bidi_streams(1)
            .expect("can't be bigger then `VarInt`")
            // TODO: handle uni streams
            .max_concurrent_uni_streams(0)
            .expect("can't be bigger then `VarInt`");
        // TODO: handle credits
        // transport.stream_receive_window(); // total bytes receive buffer for a stream: (maximum bytes allowed per stream) * (expected latency)
        // transport.receive_window(); // total bytes receive buffer for all streams of a single peer: (maximum number of streams) * (stream receive window)
        // transport.send_window(); // total bytes send buffer for all streams of a single peer
        // TODO: handle congestion, needs research
        // transport.congestion_controller_factory();
        cfg.transport = Arc::new(transport);

        // TODO: finish client certification
        // let tls_cfg = Arc::get_mut(&mut cfg.crypto).unwrap();
        // tls_cfg.set_client_certificate_verifier(Arc::new(ClientCertsWrapper(client_certs)));

        let mut endpoint_builder = Endpoint::builder();
        let _ = endpoint_builder.listen(cfg);
        let (endpoint, incoming) = endpoint_builder
            .bind(&address.into())
            .map_err(Error::BindSocket)?;

        let (sender, receiver) = flume::unbounded();

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
        sender: Sender<Connection>, /*, filter: impl Filter + 'static */
    ) -> Result<()> {
        let mut tasks = Vec::new();

        while let Some(incoming) = incoming.next().await {
            //let filter = filter.clone();
            let sender = sender.clone();

            // TODO: configurable executor
            tasks.push(tokio::spawn(async move {
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
                }) = incoming.await
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

                Result::<(), Error>::Ok(())
            }));
        }

        Ok(())
    }
}

pub struct Connection {
    connection: quinn::Connection,
    bi_streams: IncomingBiStreams,
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
