use super::{BroadcastReceiver, Channel, Management, Shutdown};
use super::{Message, Task, EXECUTOR};
use futures_util::{
    stream::{Fuse, SelectAll},
    FutureExt, StreamExt,
};
use once_cell::unsync::Lazy;
use std::{
    cell::{Ref, RefCell, RefMut},
    future::Future,
    iter,
    iter::FromIterator,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
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
        count: Arc<AtomicUsize>,
        finished: Fuse<Channel>,
    },
}

// TODO: fix Clippy
#[allow(clippy::use_self)]
impl Worker {
    thread_local!(static WORKER: Lazy<RefCell<Worker>> = Lazy::new(Worker::init));
}

macro_rules! select_task_internal {
    // TODO: fix clippy
    // passing `Worker` as a macro parameter because `clippy::use_self` was very persistent
    ($worker:ident$(, $shutdown:ident, $finished:expr)?) => {
        $worker::with_mut(|mut worker| {
            let worker = &mut *worker;

            // TODO: fix in Clippy
            #[allow(clippy::mut_mut, unused_variables)]
            futures_lite::future::block_on(async move {
                match &mut worker.schedule {
                    Schedule::Async {
                        management,
                        prio,
                        prio_steal,
                        normal,
                        normal_steal,
                    } => {
                        futures_util::select_biased![
                            $(message = worker.$shutdown.select_next_some() => Message::from(message),)?
                            message = management.select_next_some() => message,
                            message = worker.local_prio_queue.select_next_some() => message,
                            message = prio.select_next_some() => message,
                            message = prio_steal.select_next_some() => message,
                            message = worker.local_normal_queue.select_next_some() => message,
                            message = normal.select_next_some() => message,
                            message = normal_steal.select_next_some() => message,
                        ]
                    }
                    Schedule::Blocking { count: _, finished } => {
                        futures_util::select_biased![
                            $(message = worker.$shutdown.select_next_some() => Message::from(message),)?
                            message = worker.local_prio_queue.select_next_some() => message,
                            message = worker.local_normal_queue.select_next_some() => message,
                            $(message = finished.$finished() => Message::from(message),)?
                        ]
                    }
                }
            })
        })
    }
}

macro_rules! select_task {
    () => {
        // for some reason Rust doesn't wanna accept passing the `Ident` `finished` here
        select_task_internal!(Self, shutdown, select_next_some)
    };
}

macro_rules! select_task_nested {
    () => {
        select_task_internal!(Worker)
    };
}

impl Worker {
    /// # Notes
    /// This is used by the thread-local [`WORKER`] and not intended to be used otherwise.
    ///
    /// # Panics
    /// Panics if thread wasn't given a correct name.
    fn init() -> RefCell<Self> {
        // parse thread name to determine schedule
        let index = match thread::current().name().expect("no thread name set") {
            "main" => Some(0),
            "blocking" => None,
            index => {
                if let Ok(index) = index.parse() {
                    Some(index)
                } else {
                    panic!("thread name \"{}\", couldn't be parsed", index)
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
            let count = Arc::default();
            let finished = Channel::new().fuse();

            Schedule::Blocking { count, finished }
        }
    }

    /// # Notes
    /// This will block the thread until shut down, you can still call [`Task::spawn`] before calling this.
    pub(super) fn start() {
        loop {
            match select_task!() {
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
}

impl<R> Task<R> {
    fn spawn_internal<F, Q>(task: F, queue: Q) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
        Q: FnOnce(Ref<'_, Worker>) -> Channel,
    {
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        // we can't try to access the `Channel` from inside the `schedule` because `Futures` might move between threads
        let queue = Worker::with(queue);
        let (runnable, task) =
            async_task::spawn(task, move |runnable| queue.send(Message::Task(runnable)));
        runnable.schedule();
        Self(Some(task))
    }

    fn spawn_internal_local<F, Q>(task: F, queue: Q) -> Self
    where
        F: Future<Output = R> + 'static,
        Q: FnOnce(Ref<'_, Worker>) -> Channel,
    {
        let task = Self::spawn_blocking_internal(task);
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        // we can't try to access the `Channel` from inside the `schedule` because `Futures` might move between threads
        let queue = Worker::with(queue);
        let (runnable, task) =
            async_task::spawn_local(task, move |runnable| queue.send(Message::Task(runnable)));
        runnable.schedule();
        Self(Some(task))
    }

    /// This makes sure that a blocking worker keeps tabs on the amount of local tasks it has to process.
    /// If that count reaches zero it will send a [`FinishedBlocking`](Management::FinishedBlocking) [`Message`].
    ///
    /// # Notes
    /// Only use this for local tasks.
    #[allow(clippy::future_not_send)]
    fn spawn_blocking_internal(task: impl Future<Output = R>) -> impl Future<Output = R> {
        // build future that has a non-opaque type
        async fn internal<R>(
            task: impl Future<Output = R>,
            count: Option<(Arc<AtomicUsize>, Channel)>,
        ) -> R {
            let result = task.await;

            // if this is a blocking worker, subtract task count after we finish a task
            if let Some((count, finished)) = count {
                // we substract by one, if the last value was one, then we reached zero
                if count.fetch_sub(1, Ordering::SeqCst) == 1 {
                    finished.send(Message::Management(Management::FinishedBlocking))
                }
            }

            result
        }

        // if this is a blocking worker, increase the task count
        let count = Worker::with(|worker| {
            if let Schedule::Blocking { count, finished } = &worker.schedule {
                count.fetch_add(1, Ordering::SeqCst);
                Some((count.clone(), finished.get_ref().clone()))
            } else {
                None
            }
        });

        // an `Arc` is passed into the future which uses it to decrease the task count after it's finished
        // because a future can move to different threads where we loose access to the correct `Worker`
        internal(task, count)
    }

    pub fn block_on<F>(task: F) -> R
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        let task = Self::spawn_blocking_internal(task);
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        // we can't try to access the `Channel` from inside the `schedule` because `Futures` might move between threads
        let queue = Worker::with(move |worker| worker.local_prio_queue.get_ref().clone());
        let (runnable, task) =
            async_task::spawn_local(task, move |task| queue.send(Message::Task(task)));
        runnable.run();
        let mut task = Self(Some(task));

        // we are not interested in any specific `Waker`, as this is running in a loop anyway
        // the `async` yielding happens in `select_task_nested`, which is awaiting a `Message` on a `Channel`
        // `schedule` takes care of sending a message when the `Task` is ready
        let waker = futures_util::task::noop_waker();
        let mut context = Context::from_waker(&waker);

        loop {
            match task.poll_unpin(&mut context) {
                Poll::Ready(result) => break result,
                Poll::Pending => {
                    // we nest `Worker::start` here to simulate `async` yielding inside of a `block_on`
                    // TODO: encapsulate this back into `Worker::start`
                    match select_task_nested!() {
                        Message::Task(task) => {
                            task.run();
                        }
                        // TODO: log management commands
                        Message::Management(management) => match management {
                            // TODO: log that worker has successfully shutdown, but not for blocking workers
                            Management::FinishedBlocking | Management::Shutdown(Shutdown) => {
                                // `select_task_nested` specifically skips reading from `Channel`s which transmit these `Message`s
                                unreachable!("received invalid messages in `block_on`")
                            }
                        },
                    }
                }
            }
        }
    }

    pub fn spawn_prio<F>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        Self::spawn_internal(task, |worker| match &worker.schedule {
            Schedule::Async { prio, .. } => prio.get_ref().clone(),
            // in a blocking worker we want to send away non-local tasks
            // to shut down as soon as possible
            Schedule::Blocking { .. } => EXECUTOR.global_prio_injector.clone(),
        })
    }

    pub fn spawn_local_prio<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |worker| worker.local_prio_queue.get_ref().clone())
    }

    pub fn spawn<F>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        Self::spawn_internal(task, |worker| match &worker.schedule {
            Schedule::Async { normal, .. } => normal.get_ref().clone(),
            // in a blocking worker we want to send away non-local tasks
            // to shut down as soon as possible
            Schedule::Blocking { .. } => EXECUTOR.global_normal_injector.clone(),
        })
    }

    pub fn spawn_local<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |worker| worker.local_normal_queue.get_ref().clone())
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
        // we intentionally want to query the `Blocking` `Worker` inside the newly spawned thread for a `Channel`
        // otherwise we would send back the blocking `Task` to the spawning `Worker`
        let (runnable, task) = async_task::spawn(task, |task| {
            Worker::with(move |worker| worker.local_prio_queue.get_ref().send(Message::Task(task)))
        });

        // blocking tasks are spawned in a separate start
        Builder::new()
            .name(String::from("blocking"))
            .spawn(move || {
                // run task inside the new thread instead of the spawning one
                runnable.run();
                Worker::start();
            })
            .expect("failed to spawn thread");

        Self(Some(task))
    }
}
