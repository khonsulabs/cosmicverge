mod worker;

use async_task::Runnable;
use core_affinity::CoreId;
use flume::{r#async::RecvStream, Sender};
use futures_util::{stream::Stream, FutureExt, StreamExt};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::{
    future::Future,
    iter,
    pin::Pin,
    task::{Context, Poll},
    thread::Builder,
};
use worker::Worker;

static EXECUTOR: Lazy<Executor> = Lazy::new(Executor::init);

pub struct Executor {
    cores: Vec<Option<CoreId>>,
    shutdown: Broadcaster<Shutdown>,
    management: Vec<Channel>,
    local_prio_queues: Vec<Channel>,
    local_normal_queues: Vec<Channel>,
    global_prio_queues: Vec<Channel>,
    global_normal_queues: Vec<Channel>,
    global_prio_injector: Channel,
    global_normal_injector: Channel,
    #[cfg(feature = "tokio-support")]
    tokio: tokio::runtime::Runtime,
}

impl Executor {
    /// # Notes
    /// This is used by the global `EXECUTOR` and not intended to be used otherwise.
    fn init() -> Self {
        // collect `CoreId`s to pin threads
        let cores = match core_affinity::get_core_ids() {
            Some(cores) if !cores.is_empty() => cores.into_iter().map(Some).collect(),
            _ => {
                // TODO: log that we couldn't pin threads
                let cores = match num_cpus::get_physical() {
                    0 => unreachable!("no cores found"),
                    cores => cores,
                };
                vec![None; cores]
            }
        };

        // create queues
        let shutdown = Broadcaster::new();
        let management = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let local_prio_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let local_normal_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let global_prio_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let global_normal_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let global_prio_injector = Channel::new();
        let global_normal_injector = Channel::new();

        // add tokio support
        #[cfg(feature = "tokio-support")]
        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio `Runtime`");

        // build `Executor`
        Self {
            cores,
            shutdown,
            management,
            local_prio_queues,
            local_normal_queues,
            global_prio_queues,
            global_normal_queues,
            global_prio_injector,
            global_normal_injector,
            #[cfg(feature = "tokio-support")]
            tokio,
        }
    }

    /// # Notes
    /// Will shut down the current `Executor` before starting a new one.
    ///
    /// # Panics
    /// Panics if called inside a `futures_executor::block_on` context.
    pub fn start<M, R>(main: M) -> R
    where
        M: Future<Output = R> + 'static,
        R: 'static,
    {
        // shutdown before start
        // futures_executor::block_on(Self::shutdown());
        // spawn a thread for each physical CPU core except the first one
        for index in 1..EXECUTOR.cores.len() {
            Builder::new()
                .name(index.to_string())
                .spawn(Worker::start)
                .expect("failed to spawn thread");
        }

        // build `main`
        let main = Task::spawn_local_prio(async move {
            let result = main.await;
            // if main is done, force-shutdown everything else
            Self::shutdown().await;
            result
        });
        // start worker on the main thread
        Worker::start();
        // return the result of main
        futures_executor::block_on(main)
    }

    pub async fn shutdown() {
        // TODO: log useful data on shutdown:
        // - how many tasks were still unfinished
        // - panics or errors in `Worker`s
        // TODO: empty queues
        EXECUTOR.shutdown.send(Shutdown);
    }
}

enum Message {
    Task(Runnable),
    Management(Management),
}

impl From<Shutdown> for Message {
    fn from(shutdown: Shutdown) -> Self {
        Self::Management(Management::Shutdown(shutdown))
    }
}

enum Management {
    /// Shutdown `Executor`
    Shutdown(Shutdown),
    /// Blocking thread finished processing all tasks
    FinishedBlocking,
}

#[derive(Clone, Copy)]
struct Shutdown;

pub struct Task<R: 'static>(Option<async_task::Task<R>>);

impl<R> Future for Task<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.as_mut()
            .0
            .as_mut()
            .expect("task already dropped")
            .poll_unpin(context)
    }
}

impl<R> Drop for Task<R> {
    fn drop(&mut self) {
        // by default `async_task::Task` cancels on drop, we wan't to detach on drop
        self.0.take().expect("task alrady dropped").detach()
    }
}

// TODO: this will need an overhaul if alternatives to `flume` can be found:
// - split into: MPMC, SPMC, SPSC, non-`Send` SPSC
// - split Sender and Receiver, which is currently unnecessary because this is only an MPMC
#[derive(Clone)]
struct Channel {
    sender: Sender<Message>,
    receiver: RecvStream<'static, Message>,
}

impl Channel {
    fn new() -> Self {
        let (sender, receiver) = flume::unbounded();
        let receiver = receiver.into_stream();

        Self { sender, receiver }
    }

    fn send(&self, task: Message) {
        self.sender.send(task).expect("no receiver alive")
    }
}

impl Stream for Channel {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().receiver.poll_next_unpin(context)
    }
}

// TODO: this will need an overhaul if alternatives to `flume` can be found
struct Broadcaster<T: Clone>(RwLock<Vec<Sender<T>>>);

impl<T: Clone> Broadcaster<T> {
    fn new() -> Self {
        Self(RwLock::default())
    }

    fn subscribe(&self) -> BroadcastReceiver<T> {
        let (sender, receiver) = flume::unbounded();
        // locking is only done for a very short period of time
        // making async-locking probably not worth the cost
        self.0.write().push(sender);
        BroadcastReceiver(receiver.into_stream())
    }

    fn send(&self, message: T) {
        // throw out any `Sender`s in the process of sending that don't have a receiver anymore
        self.0
            .write()
            .retain(|sender| sender.send(message.clone()).is_ok());
    }
}

struct BroadcastReceiver<T: 'static>(RecvStream<'static, T>);

impl<T> Stream for BroadcastReceiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().0.poll_next_unpin(context)
    }
}
