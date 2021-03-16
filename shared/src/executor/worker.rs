use super::Channel;
use super::{Message, Task, EXECUTOR};
use async_task::Runnable;
use futures_util::{
    stream::{Fuse, SelectAll},
    StreamExt,
};
use once_cell::unsync::Lazy;
use std::{
    cell::{Ref, RefCell, RefMut},
    future::Future,
    iter,
    iter::FromIterator,
    thread,
};

pub struct Worker {
    shutdown: Fuse<Channel>,
    management: Fuse<Channel>,
    local_prio_queue: Fuse<Channel>,
    local_normal_queue: Fuse<Channel>,
    global_queues: Option<Queues>,
    #[cfg(feature = "tokio-support")]
    tokio: tokio::runtime::Handle,
}

struct Queues {
    prio: Fuse<Channel>,
    prio_steal: Fuse<SelectAll<Channel>>,
    normal: Fuse<Channel>,
    normal_steal: Fuse<SelectAll<Channel>>,
    injector: Fuse<Channel>,
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
            "main" => Some(0),
            "blocking" => None,
            index => {
                if let Ok(index) = index.parse() {
                    Some(index)
                } else {
                    panic!("thread name couldn't be parsed")
                }
            }
        };

        // pin thread to a physical CPU core
        if let Some(index) = index {
            if let Some(core_id) = EXECUTOR.cores.get(index).expect("no core found") {
                core_affinity::set_for_current(*core_id);
            }
        }

        // shutdown queue
        let shutdown = EXECUTOR.shutdown.clone().fuse();

        // management queue
        let management = EXECUTOR.management.clone().fuse();

        // build task queues
        let local_prio_queue;
        let local_normal_queue;

        // build local queues
        let global_queues = if let Some(index) = index {
            local_prio_queue = EXECUTOR
                .local_prio_queues
                .get(index)
                .expect("no local priority queue found")
                .clone()
                .fuse();
            local_normal_queue = EXECUTOR
                .local_normal_queues
                .get(index)
                .expect("no local normal queue found")
                .clone()
                .fuse();

            Some(Self::init_global_queues(index))
        } else {
            local_prio_queue = Channel::new().fuse();
            local_normal_queue = Channel::new().fuse();
            None
        };

        #[cfg(feature = "tokio-support")]
        let tokio = EXECUTOR.tokio.handle().clone();

        // build `Worker`
        RefCell::new(Self {
            shutdown,
            management,
            local_prio_queue,
            local_normal_queue,
            global_queues,
            #[cfg(feature = "tokio-support")]
            tokio,
        })
    }

    fn init_global_queues(index: usize) -> Queues {
        // split of own priority queue from others
        let mut prios = EXECUTOR.global_prio_queues.clone();
        let prio = prios
            .splice(index..=index, iter::empty())
            .next()
            .expect("no priority queue found")
            .fuse();
        let prio_steal = SelectAll::from_iter(prios).fuse();

        // split of own normal queue from others
        let mut normals = EXECUTOR.global_normal_queues.clone();
        let normal = normals
            .splice(index..=index, iter::empty())
            .next()
            .expect("no normal queue found")
            .fuse();
        let normal_steal = SelectAll::from_iter(normals).fuse();

        // injector queue
        let injector = EXECUTOR.global_injector_queue.clone().fuse();

        // build `Queues`
        Queues {
            prio,
            prio_steal,
            normal,
            normal_steal,
            injector,
        }
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

    /// # Panics
    /// Panics if called inside a `futures_executor::block_on` context.
    fn select_task() -> Message {
        Self::with_mut(|mut worker| {
            let worker = &mut *worker;

            // TODO: fix in Clippy
            #[allow(clippy::mut_mut)]
            futures_executor::block_on(async move {
                if let Some(global_queues) = &mut worker.global_queues {
                    futures_util::select_biased![
                        message = worker.shutdown.select_next_some() => message,
                        message = worker.management.select_next_some() => message,
                        message = worker.local_prio_queue.select_next_some() => message,
                        message = global_queues.prio.select_next_some() => message,
                        message = global_queues.prio_steal.select_next_some() => message,
                        message = worker.local_normal_queue.select_next_some() => message,
                        message = global_queues.normal.select_next_some() => message,
                        message = global_queues.normal_steal.select_next_some() => message,
                        message = global_queues.injector.select_next_some() => message,
                    ]
                } else {
                    futures_util::select_biased![
                        message = worker.shutdown.select_next_some() => message,
                        message = worker.management.select_next_some() => message,
                        message = worker.local_prio_queue.select_next_some() => message,
                        message = worker.local_normal_queue.select_next_some() => message,
                    ]
                }
            })
        })
    }
}

impl<R> Task<R> {
    fn spawn_internal<F, S>(task: F, schedule: S) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
        S: Fn(Runnable) + Send + Sync + 'static,
    {
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        let (runnable, task) = async_task::spawn(task, schedule);
        runnable.schedule();
        Self(Some(task))
    }

    fn spawn_internal_local<F, S>(task: F, schedule: S) -> Self
    where
        F: Future<Output = R> + 'static,
        S: Fn(Runnable) + Send + Sync + 'static,
    {
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        let (runnable, task) = async_task::spawn_local(task, schedule);
        runnable.schedule();
        Self(Some(task))
    }

    pub fn spawn_prio<F>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        Self::spawn_internal(task, |task| {
            Worker::with(move |worker| {
                if let Some(global_queues) = &worker.global_queues {
                    global_queues.prio.get_ref().send(Message::Task(task))
                } else {
                    EXECUTOR.global_injector_queue.send(Message::Task(task))
                }
            })
        })
    }

    pub fn spawn_local_prio<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |task| {
            Worker::with(move |worker| worker.local_prio_queue.get_ref().send(Message::Task(task)))
        })
    }

    pub fn spawn<F>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        Self::spawn_internal(task, |task| {
            Worker::with(move |worker| {
                if let Some(global_queues) = &worker.global_queues {
                    global_queues.normal.get_ref().send(Message::Task(task))
                } else {
                    EXECUTOR.global_injector_queue.send(Message::Task(task))
                }
            })
        })
    }

    pub fn spawn_local<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |task| {
            Worker::with(move |worker| {
                worker
                    .local_normal_queue
                    .get_ref()
                    .send(Message::Task(task))
            })
        })
    }
}