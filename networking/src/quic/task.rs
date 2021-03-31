//! Wrapper to handle closing async tasks in a concurrent way.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_channel::oneshot;
use futures_util::FutureExt;
use oneshot::Sender;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use crate::{Error, Result};

/// Wrapper to abort tasks when they are dropped.
#[derive(Clone, Debug)]
pub(super) struct Task<R, S = ()>(Arc<Mutex<Option<Inner<R, S>>>>);

/// Inner wrapper for [`Task`].
#[derive(Debug)]
struct Inner<R, S> {
    /// Async task handle.
    handle: JoinHandle<R>,
    /// Channel for close signal.
    close: Sender<S>,
}

impl<R> Task<R> {
    /// Builds a new [`Task`].
    #[allow(clippy::new_ret_no_self)]
    pub(super) fn new<T, S>(task: T, close: Sender<S>) -> Task<R, S>
    where
        T: Future<Output = R> + Send + 'static,
        T::Output: Send + 'static,
    {
        // TODO: configurable executor
        let handle = tokio::spawn(task);

        Task(Arc::new(Mutex::new(Some(Inner { handle, close }))))
    }
}

impl<R, S> Task<R, S> {
    /// Shuts down the [`Task`] by sending the close signal.
    ///
    /// # Panics
    /// Will propagate any panics that happened in the task.
    pub(super) async fn close(&self, message: S) -> Result<R> {
        let inner = self.0.lock().take().ok_or(Error::AlreadyClosed)?;

        // in the meantime the task could have panicked and dropped the receiver
        inner
            .close
            .send(message)
            .map_err(|_error| Error::AlreadyClosed)?;

        // propage any panics
        #[allow(clippy::expect_used)]
        Ok(inner.handle.await.expect("task panicked"))
    }

    /// Aborts the [`Task`].
    ///
    /// # Panics
    /// Will propagate any panics that happened in the task.
    pub(super) async fn abort(&self) -> Result<R> {
        if let Some(inner) = self.0.lock().take() {
            inner.handle.abort();
            // propage any panics
            #[allow(clippy::expect_used)]
            Ok(inner.handle.await.expect("task panicked"))
        } else {
            Err(Error::AlreadyClosed)
        }
    }
}

impl<R, S> Future for &Task<R, S> {
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[allow(clippy::expect_used)]
        self.0
            .lock()
            .as_mut()
            .map_or(Poll::Ready(Err(Error::AlreadyClosed)), |inner| {
                inner
                    .handle
                    .poll_unpin(cx)
                    // propage any panics
                    .map(|result| (Ok(result.expect("task panicked"))))
            })
    }
}
