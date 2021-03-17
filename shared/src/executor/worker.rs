use super::{BroadcastReceiver, Channel, Management, Shutdown};
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
    thread::{self, Builder},
};

pub struct Worker {
    shutdown: Fuse<BroadcastReceiver<Shutdown>>,
    local_prio_queue: Fuse<Channel>,
    local_normal_queue: Fuse<Channel>,
    schedule: Schedule,
    #[cfg(feature = "tokio-support")]
    tokio: tokio::runtime::Handle,
}

enum Schedule {
    Async {
        management: Fuse<Channel>,
        prio: Fuse<Channel>,
        prio_steal: Fuse<SelectAll<Channel>>,
        normal: Fuse<Channel>,
        normal_steal: Fuse<SelectAll<Channel>>,
    },
    Blocking {
        tasks: RefCell<usize>,
        finished: Fuse<Channel>,
    },
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
    /// Panics if thread wasn't given a correct name.
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
        let shutdown = EXECUTOR.shutdown.subscribe().fuse();

        // build local queues
        let local_prio_queue;
        let local_normal_queue;

        // async workers get their local queues from the executor
        if let Some(index) = index {
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
        }
        // blocking workers make their own local queues
        else {
            local_prio_queue = Channel::new().fuse();
            local_normal_queue = Channel::new().fuse();
        }

        // build other queues
        let schedule = Self::init_schedule(index);

        // add tokio support
        #[cfg(feature = "tokio-support")]
        let tokio = EXECUTOR.tokio.handle().clone();

        // build `Worker`
        RefCell::new(Self {
            shutdown,
            local_prio_queue,
            local_normal_queue,
            schedule,
            #[cfg(feature = "tokio-support")]
            tokio,
        })
    }

    fn init_schedule(index: Option<usize>) -> Schedule {
        if let Some(index) = index {
            // get management queue for this thread
            let management = EXECUTOR
                .management
                .get(index)
                .expect("no management queue found")
                .clone()
                .fuse();

            // get priority queues to steal from
            let mut prios = EXECUTOR.global_prio_queues.clone();
            // split off own priority queue
            let prio = prios
                .splice(index..=index, iter::empty())
                .next()
                .expect("no priority queue found")
                .fuse();
            // add priority injector queue
            let mut prio_steal = SelectAll::from_iter(prios);
            prio_steal.push(EXECUTOR.global_prio_injector.clone());
            let prio_steal = prio_steal.fuse();

            // get normal queues to steal from
            let mut normals = EXECUTOR.global_normal_queues.clone();
            // split off own normal queue
            let normal = normals
                .splice(index..=index, iter::empty())
                .next()
                .expect("no normal queue found")
                .fuse();
            // add normal injector queue
            let mut normal_steal = SelectAll::from_iter(normals);
            normal_steal.push(EXECUTOR.global_normal_injector.clone());
            let normal_steal = normal_steal.fuse();

            // build `Queues`
            Schedule::Async {
                management,
                prio,
                prio_steal,
                normal,
                normal_steal,
            }
        } else {
            let tasks = RefCell::new(0);
            let finished = Channel::new().fuse();
            Schedule::Blocking { tasks, finished }
        }
    }

    /// # Notes
    /// This will block the thread until shut down, you can still `Task::spawn` before calling this.
    pub(super) fn start() {
        loop {
            match Self::select_task() {
                Message::Task(task) => {
                    task.run();
                }
                // TODO: log management commands
                Message::Management(management) => match management {
                    // TODO: log that worker has successfully shutdown, but not for blocking workers
                    Management::FinishedBlocking | Management::Shutdown(Shutdown) => break,
                },
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
                match &mut worker.schedule {
                    Schedule::Async {
                        management,
                        prio,
                        prio_steal,
                        normal,
                        normal_steal,
                    } => {
                        futures_util::select_biased![
                            message = worker.shutdown.select_next_some() => Message::from(message),
                            message = management.select_next_some() => message,
                            message = worker.local_prio_queue.select_next_some() => message,
                            message = prio.select_next_some() => message,
                            message = prio_steal.select_next_some() => message,
                            message = worker.local_normal_queue.select_next_some() => message,
                            message = normal.select_next_some() => message,
                            message = normal_steal.select_next_some() => message,
                        ]
                    }
                    Schedule::Blocking { tasks: _, finished } => {
                        futures_util::select_biased![
                            message = worker.shutdown.select_next_some() => Message::from(message),
                            message = worker.local_prio_queue.select_next_some() => message,
                            message = worker.local_normal_queue.select_next_some() => message,
                            message = finished.select_next_some() => message,
                        ]
                    }
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
        let task = Self::spawn_blocking_internal(task);
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
            Worker::with(move |worker| match &worker.schedule {
                Schedule::Async { prio, .. } => prio.get_ref().send(Message::Task(task)),
                // in a blocking worker we want to send away non-local tasks
                // to shut down as soon as possible
                Schedule::Blocking { .. } => {
                    EXECUTOR.global_prio_injector.send(Message::Task(task))
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
            Worker::with(move |worker| match &worker.schedule {
                Schedule::Async { normal, .. } => normal.get_ref().send(Message::Task(task)),
                // in a blocking worker we want to send away non-local tasks
                // to shut down as soon as possible
                Schedule::Blocking { .. } => {
                    EXECUTOR.global_normal_injector.send(Message::Task(task))
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

    pub fn spawn_blocking<F, S>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        let task = Self::spawn_blocking_internal(task);
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        let (runnable, task) = async_task::spawn(task, |task| {
            Worker::with(move |worker| worker.local_prio_queue.get_ref().send(Message::Task(task)))
        });

        // blocking tasks are spawned in a separate start
        Builder::new()
            .name(String::from("blocking"))
            .spawn(move || {
                // schedule task inside the new thread instead of the spawning one
                runnable.schedule();
                Worker::start();
            })
            .expect("failed to spawn thread");

        Self(Some(task))
    }

    /// This makes sure that a blocking worker keeps tabs on the amount of local tasks it has to process.
    /// If that count reaches zero it will send a [`FinishedBlocking`](Management::FinishedBlocking) message.
    ///
    /// # Notes
    /// Only use this in a local task.
    #[allow(clippy::future_not_send)]
    async fn spawn_blocking_internal(task: impl Future<Output = R>) -> R {
        // increase the count by one
        Worker::with(|worker| {
            if let Schedule::Blocking { tasks, .. } = &worker.schedule {
                *tasks.borrow_mut() += 1;
            }
        });

        let result = task.await;

        // decrease count after task is done
        Worker::with(|worker| {
            if let Schedule::Blocking { tasks, finished } = &worker.schedule {
                *tasks.borrow_mut() -= 1;

                // if no local tasks are left, tell thread to shut down
                if *tasks.borrow() == 0 {
                    finished
                        .get_ref()
                        .send(Message::Management(Management::FinishedBlocking))
                }
            }
        });

        result
    }
}
