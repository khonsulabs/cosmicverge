//! [`Receiver`] part of a stream.

use std::{
    convert::TryFrom,
    fmt::{self, Debug, Formatter},
    mem::{self, size_of},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_channel::oneshot;
use futures_util::{
    future::{BoxFuture, FutureExt},
    stream::Stream,
    StreamExt,
};
use parking_lot::Mutex;
use quinn::{Chunk, VarInt};
use serde::de::DeserializeOwned;

use super::{StreamExtExt, Task};
use crate::{Error, Result};

/// Used to receive data from a stream.
pub struct Receiver<T: 'static> {
    /// Send [`Deserialize`](serde::Deserialize)d data to the sending task.
    receiver: flume::r#async::RecvStream<'static, Result<T>>,
    /// [`Task`] handle that does the receiving from the stream.
    task: Task<Result<()>>,
}

impl<T> Debug for Receiver<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver")
            .field("receiver", &String::from("RecvStream<Result<T>>"))
            .field("task", &self.task)
            .finish()
    }
}

impl<T> Receiver<T> {
    /// Builds a new [`Receiver`] from a raw [`quinn`] type. Spawns a task that
    /// receives data from the stream.
    pub(super) fn new(mut stream_receiver: quinn::RecvStream) -> Self
    where
        T: DeserializeOwned + Send,
    {
        // receiver channels
        let (sender, receiver) = flume::unbounded();
        let receiver = receiver.into_stream();

        // `Task` handling `Receiver`
        let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();
        let task = Task::new(
            async move {
                /// Help Group Messages
                enum Message {
                    /// Data arrived from stream.
                    Data(Bytes),
                    /// [`Receiver`] asked to close.
                    Close,
                }

                let stream_receiver = Arc::new(Mutex::new(stream_receiver));
                let mut stream = RecvStream::new(Arc::clone(&stream_receiver)).fuse_last();

                let mut length = 0;
                // 1480 bytes is a default MTU size configured by quinn-proto
                let mut data = BytesMut::with_capacity(1480);

                // TODO: fix clippy
                #[allow(clippy::mut_mut, clippy::panic)]
                while let Some(message) = futures_util::select_biased! {
                    message = stream.select_next_some() => message.transpose()?.map(Message::Data),
                    /*message = stream_receiver.read_chunk(usize::MAX, true).fuse() => {
                        message.map_err(Error::Read)?.map(|Chunk { bytes, .. }| Message::Data(bytes))
                    }*/
                    shutdown = shutdown_receiver => shutdown.ok().map(|_| Message::Close),
                    complete => unreachable!("stream should have ended when `receiver` returned `None`"),
                } {
                    match message {
                        Message::Data(bytes) => {
                            // reserves enough space to put in incoming bytes
                            data.reserve(bytes.len());
                            data.put(bytes);

                            // if we don't have a length already and there is enough to aquire it
                            if length == 0 && data.len() == size_of::<u64>() {
                                #[allow(clippy::expect_used)]
                                {
                                    // aquire the length by reading the first 8 bytes (u64)
                                    length = usize::try_from(data.get_uint_le(size_of::<u64>()))
                                        .expect("not a 64-bit system");
                                }
                            }

                            // if we have a length and the data we gathered fullfills it
                            if length != 0 && data.len() >= length {
                                // split of the correct amoutn of data from what we have gathered
                                // until now
                                let data = data.split_to(length).reader();
                                // reset the length so the condition above works again
                                length = 0;

                                // deserialize data
                                // TODO: configure bincode, for example make it bounded
                                #[allow(box_pointers)]
                                let data = bincode::deserialize_from::<_, T>(data)
                                    .map_err(|error| Error::Deserialize(Arc::new(*error)));

                                // if there is no receiver, it means that we dropped the last
                                // `Receiver`
                                if sender.send(data).is_err() {
                                    break;
                                }
                            }
                        }
                        Message::Close => {
                            stream_receiver
                                .lock()
                                .stop(VarInt::from_u32(0))
                                .map_err(|_error| Error::AlreadyClosed)?;
                        }
                    }
                }

                Ok(())
            },
            shutdown_sender,
        );

        Self { receiver, task }
    }

    /// Wait for the [`Receiver`] part of the stream to finish gracefully.
    ///
    /// This can only be achieved through the peer's
    /// [`Sender::finish`](crate::Sender::finish) or an error.
    ///
    /// # Errors
    /// This can only return [`Error::AlreadyClosed`] as an [`Err`], if it was
    /// already finished, but if it isn't, there could be errors queued up
    /// in the [`Receiver`]:
    /// - [`Error::Read`] if the [`Receiver`] failed to read from the stream
    pub async fn finish(&self) -> Result<()> {
        (&self.task).await?
    }

    /// Close the [`Receiver`] immediately.
    ///
    /// To close a [`Receiver`] gracefully use [`Receiver::finish`].
    ///
    /// # Errors
    /// This can only return [`Error::AlreadyClosed`] as an [`Err`], if it was
    /// already closed, but there could be other errors queued up in the
    /// [`Receiver`]:
    /// - [`Error::Read`] if the [`Receiver`] failed to read from the stream
    pub async fn close(&self) -> Result<()> {
        self.task.close(()).await?
    }
}

impl<T> Stream for Receiver<T> {
    type Item = Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}

struct RecvStream(BoxFuture<'static, (Arc<Mutex<quinn::RecvStream>>, Result<Option<Bytes>>)>);

impl RecvStream {
    fn new(stream: Arc<Mutex<quinn::RecvStream>>) -> Self {
        Self(Self::future(stream).boxed())
    }

    async fn future(
        stream: Arc<Mutex<quinn::RecvStream>>,
    ) -> (Arc<Mutex<quinn::RecvStream>>, Result<Option<Bytes>>) {
        let mut lock = stream.lock();
        let result = lock
            .read_chunk(usize::MAX, true)
            .await
            .map(|result| result.map(|Chunk { bytes, .. }| bytes))
            .map_err(Error::Read);
        drop(lock);
        (stream, result)
    }
}

impl Stream for RecvStream {
    type Item = Result<Bytes>;

    #[allow(box_pointers)]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.0.poll_unpin(cx) {
            Poll::Ready((stream, bytes)) => {
                let _old = mem::replace(&mut self.0, Self::future(stream).boxed());
                Poll::Ready(bytes.transpose())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
