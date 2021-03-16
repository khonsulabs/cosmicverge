mod worker;

use std::{
    future::Future,
    iter::{self},
    pin::Pin,
    task::{Context, Poll},
};

use async_task::Runnable;
use core_affinity::CoreId;
use flume::{r#async::RecvStream, Sender};
use futures_util::{stream::Stream, FutureExt, StreamExt};
use once_cell::sync::OnceCell;
use std::thread;
use worker::Worker;

static EXECUTOR: OnceCell<Executor> = OnceCell::new();

pub struct Executor {
    cores: Vec<Option<CoreId>>,
    shutdown: Channel,
    management: Channel,
    local_prio_queues: Vec<Channel>,
    local_normal_queues: Vec<Channel>,
    global_prio_queues: Vec<Channel>,
    global_normal_queues: Vec<Channel>,
}

impl Executor {
    /// # Panics
    /// If `Executor` is already initialized, this will panic.
    fn init() -> &'static Self {
        // collect `CoreId`s to pin threads
        let cores = match core_affinity::get_core_ids() {
            Some(cores) if !cores.is_empty() => cores.into_iter().map(Some).collect(),
            _ => {
                // TODO: log that we couldn't pin threads
                let cores = match num_cpus::get_physical() {
                    0 => {
                        // TODO: log that we couldn't find any cores
                        1
                    }
                    cores => cores,
                };
                vec![None; cores]
            }
        };

        // create queues
        let shutdown = Channel::new();
        let management = Channel::new();
        let local_prio_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let local_normal_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let global_prio_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();
        let global_normal_queues = iter::repeat_with(Channel::new).take(cores.len()).collect();

        // TODO: improve with https://github.com/matklad/once_cell/pull/148
        // build `Executor`
        EXECUTOR
            .set(Self {
                cores,
                shutdown,
                management,
                local_prio_queues,
                local_normal_queues,
                global_prio_queues,
                global_normal_queues,
            })
            .map_err(|_| ())
            .expect("`EXECUTOR` already set");
        EXECUTOR.get().expect("`EXECUTOR` not set")
    }

    /// # Panics
    /// If `Executor` is already started, this will panic.
    pub fn start<M, R>(main: M) -> R
    where
        M: Future<Output = R> + 'static,
        R: 'static,
    {
        let executor = Self::init();

        // spawn a thread for each physical CPU core except the first one
        for index in 1..executor.cores.len() {
            thread::spawn(move || {
                // spin up `Worker`
                Worker::init(executor, index);
                Worker::start();
            });
        }

        // spin up `Worker` in the first core
        Worker::init(executor, 0);
        // start `main`
        let main = Task::spawn_local_prio(async move {
            let result = main.await;
            // if main is done, force-shutdown everything else
            Self::shutdown().await;
            result
        });
        Worker::start();
        // return the result of main
        futures_executor::block_on(main)
    }

    /// Notes:
    /// This will do nothing if `Executor` isn't initialized.
    pub async fn shutdown() {
        // TODO: log useful data on shhutdown:
        // - how many tasks were still unfinished
        // - panics or errors in `Worker`s
        if let Some(executor) = EXECUTOR.get() {
            executor.shutdown.send(Message::Shutdown)
        }
    }
}

enum Message {
    Task(Runnable),
    Management(Management),
    Shutdown,
}

enum Management {}

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
        Self {
            sender,
            receiver: receiver.into_stream(),
        }
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
