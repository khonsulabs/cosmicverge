//! Helper types that don't fit anywhere else.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{
    stream::{FusedStream, Stream},
    StreamExt as _,
};

/// Wrapper around any [`Stream`] to enable a type of
/// [`fusing`](futures_util::StreamExt::fuse) that allows for a last item to be
/// returned when the stream is exhausted.
pub(super) trait StreamExtExt {
    /// Wraps the item of the stream into an [`Option`] that allows a last item,
    /// [`Some(None)`], to be returned when the stream is exhausted.
    fn fuse_last(self) -> FusedLast<Self>
    where
        Self: Sized + Stream + Unpin,
    {
        FusedLast::new(self)
    }
}

impl<T: Stream> StreamExtExt for T {}

/// Returned by [`fuse_last`](StreamExtExt::fuse_last).
pub(super) struct FusedLast<S> {
    /// The stream [`FusedLast`] is wrapped around.
    stream: S,
    /// Tracks the state of exhaustion for the [`Stream`] and [`FusedLast`].
    state: State,
}

/// Tracks the state of [`FusedLast`].
enum State {
    /// [`Stream`] is still producing items.
    Active,
    /// [`Stream`] is done, but [`FusedLast`] will still produce one last item.
    Done,
    /// [`Stream`] and [`FusedLast`] are done.
    Finished,
}

impl<S> FusedLast<S> {
    /// Builds a new [`FusedLast`].
    const fn new(stream: S) -> Self {
        Self {
            stream,
            state: State::Active,
        }
    }
}

impl<S: Stream + Unpin> Stream for FusedLast<S> {
    type Item = Option<<S as Stream>::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx).map(|item| {
            if item.is_none() {
                match self.state {
                    State::Active => {
                        self.state = State::Done;
                        cx.waker().wake_by_ref();
                        Some(None)
                    }
                    State::Done => {
                        self.state = State::Finished;
                        None
                    }
                    State::Finished => None,
                }
            } else {
                Some(item)
            }
        })
    }
}

impl<S: Stream + Unpin> FusedStream for FusedLast<S> {
    fn is_terminated(&self) -> bool {
        match self.state {
            State::Active | State::Done => false,
            State::Finished => true,
        }
    }
}
