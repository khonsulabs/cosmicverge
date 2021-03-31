//! [`Connection`]s hold a connection to a peer in an
//! [`Endpoint`](crate::Endpoint).
//!
//!
//! A single [`Connection`] can have multiple streams, streams consist of a
//! [`Sender`] and [`Receiver`], which can send and receive messages on that
//! stream.
//!
//! You can use [`open_stream`](Connection::open_stream) to open a stream.

mod incoming;
mod receiver;
mod sender;

use std::{
    fmt::{self, Debug, Formatter},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use flume::r#async::RecvStream;
use futures_channel::oneshot;
use futures_util::{stream, StreamExt};
// TODO: fix lint or allow it, this is horrible
#[allow(unreachable_pub)]
pub use incoming::Incoming;
use quinn::{IncomingBiStreams, VarInt};
#[allow(unreachable_pub)]
pub use receiver::Receiver;
#[allow(unreachable_pub)]
pub use sender::Sender;
use serde::{de::DeserializeOwned, Serialize};

use super::{StreamExtExt, Task};
use crate::{Error, Result};

/// Represents an open connection to a server or client. Receives [`Incoming`]
/// streams through [`Stream`](stream::Stream).
#[derive(Clone)]
pub struct Connection {
    /// Initiate new connections or close socket.
    connection: quinn::Connection,
    /// Receive incoming streams.
    receiver: RecvStream<'static, Result<Incoming>>,
    /// [`Task`] handling new incoming streams.
    task: Arc<Task<()>>,
}

/// Holds [`Task`]s that handle sending and receiving data for a stream.
#[derive(Debug)]
struct Stream {
    /// [`Task`] handling sending data.
    sender_task: Task<Result<()>>,
    /// [`Task`] handling receiving data.
    receiver_task: Task<Result<()>>,
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("connection", &self.connection)
            .field("receiver", &String::from("RecvStream<Result<Incoming>>"))
            .field("task", &self.task)
            .finish()
    }
}

impl Connection {
    /// Builds a new [`Connection`] from raw [`quinn`] types.
    pub(super) fn new(connection: quinn::Connection, bi_streams: IncomingBiStreams) -> Self {
        // channels for passing down new `Incoming` `Connection`s
        let (sender, receiver) = flume::unbounded();
        let receiver = receiver.into_stream();

        // `Task` handling incoming streams
        let task = {
            let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();
            Arc::new(Task::new(
                async move {
                    let mut bi_streams = bi_streams.fuse_last();

                    // TODO: fix clippy
                    #[allow(clippy::mut_mut, clippy::panic)]
                    while let Some(result) = futures_util::select_biased! {
                        connecting = bi_streams.select_next_some() => connecting,
                        _ = shutdown_receiver => None,
                        complete => unreachable!("stream should have ended when `receiver` returned `None`"),
                    } {
                        let incoming = result
                            .map(|(sender, receiver)| Incoming::new(sender, receiver))
                            .map_err(Error::ReceiveStream);

                        // if there is no receiver, it means that we dropped the last `Connection`
                        if sender.send(incoming).is_err() {
                            break;
                        }
                    }
                },
                shutdown_sender,
            ))
        };

        Self {
            connection,
            receiver,
            task,
        }
    }

    /// The peer's address. Clients may change addresses at will, e.g. when
    /// switching to a cellular internet connection.
    #[must_use]
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Open a stream on this [`Connection`], allowing to send data back and
    /// forth.
    ///
    /// # Errors
    /// [`Error::OpenStream`] if opening a stream failed.
    pub async fn open_stream<T: DeserializeOwned + Serialize + Send + 'static>(
        &self,
    ) -> Result<(Sender<T>, Receiver<T>)> {
        let (sender, receiver) = self.connection.open_bi().await.map_err(Error::OpenStream)?;

        let sender = Sender::new(sender);
        let receiver = Receiver::new(receiver);

        Ok((sender, receiver))
    }

    /// Close the [`Connection`] immediately.
    ///
    /// To close a [`Connection`] gracefully use [`Sender::finish`], the
    /// [`Receiver`] can't be gracefull closed from the receiving end.
    ///
    /// # Errors
    /// [`Error::AlreadyClosed`] if it was already closed.
    pub async fn close(&self) -> Result<()> {
        self.connection.close(VarInt::from_u32(0), &[]);
        self.task.abort().await
    }
}

impl stream::Stream for Connection {
    type Item = Result<Incoming>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}
