mod client;
mod connection;
mod server;
mod transport;

use std::net::{SocketAddr, ToSocketAddrs};

// TODO: fix lint or allow it, this is horrible
#[allow(unreachable_pub)]
pub use client::Client;
#[allow(unreachable_pub)]
pub use connection::{Connection, Receiver, Sender};
#[allow(unreachable_pub)]
pub use server::Server;
use transport::transport;

use crate::{Error, Result};

/// TODO: docs
fn parse_socket(address: impl ToSocketAddrs) -> Result<SocketAddr> {
    let mut addresses = address.to_socket_addrs().map_err(Error::ParseAddress)?;
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
