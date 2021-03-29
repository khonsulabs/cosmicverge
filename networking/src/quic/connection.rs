use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{stream::Stream, StreamExt};
use quinn::{IncomingBiStreams, SendStream};

use crate::{Error, Result};

/// TODO: docs
#[derive(Debug)]
pub struct Connection {
    /// Initiate new connections or close socket.
    pub(super) connection: quinn::Connection,
    /// Receive incoming streams.
    pub(super) bi_streams: IncomingBiStreams,
}

impl Connection {
    /// The peer's address.
    /// Clients may change addresses at will, e.g. when switching to a cellular internet connection.
    #[must_use]
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }
}

impl Stream for Connection {
    type Item = Result<(Sender, Receiver)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.bi_streams
            .poll_next_unpin(cx)
            .map_ok(|(sender, receiver)| (Sender(sender), Receiver(receiver)))
            .map_err(Error::IncomingStream)
    }
}

/// TODO: docs
#[derive(Debug)]
pub struct Sender(SendStream);

/// TODO: docs
#[derive(Debug)]
pub struct Receiver(quinn::RecvStream);
