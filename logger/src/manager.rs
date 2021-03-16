use std::sync::Arc;

use flume::{Receiver, Sender};
use futures::{future::BoxFuture, FutureExt};

use crate::{backend::Backend, Log};

#[derive(Default, Debug)]
pub struct Manager {
    backends: Vec<Box<dyn Backend>>,
}

impl Manager {
    pub fn with_backend<B: Backend + 'static>(mut self, backend: B) -> Self {
        self.backends.push(Box::new(backend));
        self
    }

    #[must_use]
    pub fn launch<F: FnOnce(BoxFuture<'static, ()>)>(self, spawner: F) -> Sender<Arc<Log>> {
        let (sender, receiver) = flume::unbounded();

        spawner(self.run(receiver).boxed());

        sender
    }

    async fn run(mut self, receiver: Receiver<Arc<Log>>) {
        while let Ok(log) = receiver.recv_async().await {
            futures::future::join_all(
                self.backends
                    .iter_mut()
                    .map(|backend| backend.process_log(&log)),
            )
            .await
            .into_iter()
            .collect::<Result<Vec<_>, anyhow::Error>>()
            .expect("Error communicating with logging backends");
        }
    }
}
