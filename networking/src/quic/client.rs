use std::{net::ToSocketAddrs, sync::Arc};

use quinn::{ClientConfigBuilder, Endpoint, NewConnection};

use crate::{Certificate, Connection, Error, Result};

/// TODO: docs
#[derive(Clone, Debug)]
pub struct Client {
    /// Initiate new connections or close socket.
    endpoint: Endpoint,
}

impl Client {
    /// TODO: improve docs
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one address
    /// - [`Error::Certificate`] if the [`Certificate`] couldn't be parsed
    /// - [`Error::InvalidCertificate`] if the [`Certificate`] couldn't be added as a certificate authority
    /// - [`Error::BindSocket`] if the socket couldn't be bound to the given `address`
    #[allow(clippy::unwrap_in_result)]
    pub fn new<A: ToSocketAddrs>(address: A, certificate: &Certificate) -> Result<Self> {
        let address = super::parse_socket(address)?;

        let certificate =
            quinn::Certificate::from_der(&certificate.0).map_err(Error::Certificate)?;

        let mut cfg_builder = ClientConfigBuilder::default();
        let _ = cfg_builder
            .add_certificate_authority(certificate)
            .map_err(Error::InvalidCertificate)?;
        let mut cfg = cfg_builder.build();

        let transport = super::transport();
        cfg.transport = Arc::new(transport);

        let mut endpoint_builder = Endpoint::builder();
        let _ = endpoint_builder.default_client_config(cfg);
        let (endpoint, _) = endpoint_builder.bind(&address).map_err(Error::BindSocket)?;

        Ok(Self { endpoint })
    }

    /// TODO: docs
    ///
    /// # Errors
    /// - [`Error::ParseAddress`] if the `address` couldn't be parsed
    /// - [`Error::MultipleAddresses`] if the `address` contained more then one address
    /// - [`Error::Connect`] if no connection to the given `address` could be established
    /// - [`Error::Connecting`] if the connection to the given `address` failed
    pub async fn connect<A: ToSocketAddrs>(
        &self,
        address: A,
        server_name: &str,
    ) -> Result<Connection> {
        let address = super::parse_socket(address)?;

        let connecting = self
            .endpoint
            .connect(&address, server_name)
            .map_err(Error::Connect)?;

        let NewConnection {
            connection,
            bi_streams,
            ..
        } = connecting.await.map_err(Error::Connecting)?;

        Ok(Connection {
            connection,
            bi_streams,
        })
    }
}
