use super::{Message, Task, EXECUTOR};
use std::{
    cell::{Ref, RefCell, RefMut},
    future::Future,
    iter,
    iter::FromIterator,
    thread,
};

use super::Channel;
use futures_util::{
    stream::{Fuse, SelectAll},
    StreamExt,
};
use once_cell::unsync::Lazy;

pub struct Worker {
    shutdown: Fuse<Channel>,
    management: Fuse<Channel>,
    local_prio_queue: Fuse<Channel>,
    global_prio_queue: Fuse<Channel>,
    global_prio_steal: Fuse<SelectAll<Channel>>,
    local_normal_queue: Fuse<Channel>,
    global_normal_queue: Fuse<Channel>,
    global_normal_steal: Fuse<SelectAll<Channel>>,
}

// TODO: fix Clippy
#[allow(clippy::use_self)]
impl Worker {
    thread_local!(static WORKER: Lazy<RefCell<Worker>> = Lazy::new(Worker::init));
}

impl Worker {
    /// # Notes
    /// This is used by the thread-local `WORKER` and not intended to be used otherwise.
    ///
    /// # Panics
    /// Panics if thread wasn't given a name.
    fn init() -> RefCell<Self> {
        let index = match thread::current().name().expect("no thread name set") {
            "main" => 0,
            index => {
                if let Ok(index) = index.parse() {
                    index
                } else {
                    panic!("thread name couldn't be parsed")
                }
            }
        };

        // pin thread to a physical CPU core
        if let Some(core_id) = EXECUTOR.cores.get(index).expect("no core found") {
            core_affinity::set_for_current(*core_id);
        }

        // shutdown queue
        let shutdown = EXECUTOR.shutdown.clone().fuse();

        // management queue
        let management = EXECUTOR.management.clone().fuse();

        // build local queues
        let local_prio_queue = EXECUTOR
            .local_prio_queues
            .get(index)
            .expect("no local priority queue found")
            .clone()
            .fuse();
        let local_normal_queue = EXECUTOR
            .local_normal_queues
            .get(index)
            .expect("no local normal queue found")
            .clone()
            .fuse();

        // split of own priority queue from others
        let mut global_prio_steal = EXECUTOR.global_prio_queues.clone();
        let global_prio_queue = global_prio_steal
            .splice(index..=index, iter::empty())
            .next()
            .expect("no priority queue found")
            .fuse();
        let global_prio_steal = SelectAll::from_iter(global_prio_steal).fuse();

        // split of own normal queue from others
        let mut global_normal_steal = EXECUTOR.global_normal_queues.clone();
        let global_normal_queue = global_normal_steal
            .splice(index..=index, iter::empty())
            .next()
            .expect("no normal queue found")
            .fuse();
        let global_normal_steal = SelectAll::from_iter(global_normal_steal).fuse();

        // build `Worker`
        RefCell::new(Self {
            shutdown,
            management,
            local_prio_queue,
            global_prio_queue,
            global_prio_steal,
            local_normal_queue,
            global_normal_queue,
            global_normal_steal,
        })
    }

    /// # Notes
    /// This will block the thread until shutdown, you can still `Task::spawn` before calling this.
    pub(super) fn start() {
        loop {
            match Self::select_task() {
                Message::Task(task) => {
                    task.run();
                }
                // TODO: log management commands
                Message::Management(_management) => todo!(),
                // TODO: log that worker has successfully shutdown
                Message::Shutdown => break,
            }
        }
    }

    fn with<R>(fun: impl FnOnce(Ref<'_, Self>) -> R) -> R {
        Self::WORKER.with(|worker| fun(worker.borrow()))
    }

    fn with_mut<R>(fun: impl Fn(RefMut<'_, Self>) -> R) -> R {
        Self::WORKER.with(|worker| fun(worker.borrow_mut()))
    }

    fn select_task() -> Message {
        Self::with_mut(|mut worker| {
            let worker = &mut *worker;

            // TODO: fix in Clippy
            #[allow(clippy::mut_mut)]
            futures_executor::block_on(async move {
                futures_util::select_biased![
                    message = worker.shutdown.select_next_some() => message,
                    message = worker.management.select_next_some() => message,
                    message = worker.local_prio_queue.select_next_some() => message,
                    message = worker.global_prio_queue.select_next_some() => message,
                    message = worker.global_prio_steal.select_next_some() => message,
                    message = worker.local_normal_queue.select_next_some() => message,
                    message = worker.global_normal_queue.select_next_some() => message,
                    message = worker.global_normal_steal.select_next_some() => message,
                ]
            })
        })
    }
}

impl<R: 'static> Task<R> {
    pub fn spawn_prio<F>(future: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        let (runnable, task) = async_task::spawn(future, |task| {
            Worker::with(move |worker| worker.global_prio_queue.get_ref().send(Message::Task(task)))
        });
        runnable.schedule();
        Self(Some(task))
    }

    pub fn spawn_local_prio<F>(future: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        let (runnable, task) = async_task::spawn_local(future, |task| {
            Worker::with(move |worker| worker.local_prio_queue.get_ref().send(Message::Task(task)))
        });
        runnable.schedule();
        Self(Some(task))
    }

    pub fn spawn<F>(future: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        let (runnable, task) = async_task::spawn(future, |task| {
            Worker::with(move |worker| {
                worker
                    .global_normal_queue
                    .get_ref()
                    .send(Message::Task(task))
            })
        });
        runnable.schedule();
        Self(Some(task))
    }

    pub fn spawn_local<F>(future: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        let (runnable, task) = async_task::spawn_local(future, |task| {
            Worker::with(move |worker| {
                worker
                    .local_normal_queue
                    .get_ref()
                    .send(Message::Task(task))
            })
        });
        runnable.schedule();
        Self(Some(task))
    }
}
