use super::{BroadcastReceiver, Channel, Management, Shutdown};
use super::{Message, Task, EXECUTOR};
use futures_util::{
    stream::{Fuse, SelectAll},
    FutureExt, StreamExt,
};
use once_cell::{sync::OnceCell, unsync::Lazy};
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
    runtime: Runtime,
    #[cfg(feature = "tokio-support")]
    tokio: tokio::runtime::Handle,
}

enum Runtime {
    Async {
        management: Fuse<Channel>,
        local_prio: Fuse<Channel>,
        global_prio: Fuse<Channel>,
        steal_prio: Fuse<SelectAll<Channel>>,
        local_normal: Fuse<Channel>,
        global_normal: Fuse<Channel>,
        steal_normal: Fuse<SelectAll<Channel>>,
    },
    Blocking {
        count: Arc<AtomicUsize>,
        local_prio: Fuse<Channel>,
        local_normal: Fuse<Channel>,
        finished: Fuse<Channel>,
    },
    Alien,
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
                match &mut worker.runtime {
                    Runtime::Async {
                        management,
                        local_prio,
                        global_prio,
                        steal_prio,
                        local_normal,
                        global_normal,
                        steal_normal,
                    } => {
                        futures_util::select_biased![
                            $(message = worker.$shutdown.select_next_some() => Message::from(message),)?
                            message = management.select_next_some() => message,
                            message = local_prio.select_next_some() => message,
                            message = global_prio.select_next_some() => message,
                            message = steal_prio.select_next_some() => message,
                            message = local_normal.select_next_some() => message,
                            message = global_normal.select_next_some() => message,
                            message = steal_normal.select_next_some() => message,
                        ]
                    }
                    Runtime::Blocking { local_prio, local_normal, finished, .. } => {
                        futures_util::select_biased![
                            $(message = worker.$shutdown.select_next_some() => Message::from(message),)?
                            message = local_prio.select_next_some() => message,
                            message = local_normal.select_next_some() => message,
                            $(message = finished.$finished() => Message::from(message),)?
                        ]
                    }
                    Runtime::Alien { .. } => unreachable!("`Worker` started in an alien runtime")
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
    fn init() -> RefCell<Self> {
        // parse thread name to determine `Runtime`
        let runtime = match thread::current().name().expect("no thread name set") {
            "main" => Runtime::init_async(0),
            "blocking" => Runtime::init_blocking(),
            index => {
                if let Ok(index) = index.parse() {
                    Runtime::init_async(index)
                } else {
                    Runtime::Alien
                }
            }
        };

        // shutdown queue
        let shutdown = EXECUTOR.shutdown.subscribe().fuse();

        // add tokio support
        #[cfg(feature = "tokio-support")]
        let tokio = EXECUTOR.tokio.handle().clone();

        // build `Worker`
        RefCell::new(Self {
            shutdown,
            runtime,
            #[cfg(feature = "tokio-support")]
            tokio,
        })
    }

    /// # Notes
    /// This will block the thread until shut down, you can still call [`Task::spawn`] before calling this.
    ///
    /// # Panics
    /// Panics if started in an alien thread not spawned by the [`Executor`].
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

impl Runtime {
    fn init_async(index: usize) -> Self {
        // pin thread to a physical CPU core
        if let Some(core_id) = EXECUTOR.cores.get(index).expect("no core found") {
            core_affinity::set_for_current(*core_id);
        }

        // get local queues forr this thread
        let local_prio = EXECUTOR
            .local_prio_queues
            .get(index)
            .expect("no local priority queue found")
            .clone()
            .fuse();
        let local_normal = EXECUTOR
            .local_normal_queues
            .get(index)
            .expect("no local normal queue found")
            .clone()
            .fuse();

        // get management queue for this thread
        let management = EXECUTOR
            .management
            .get(index)
            .expect("no management queue found")
            .clone()
            .fuse();

        // get priority queues to steal from
        let mut global_prios = EXECUTOR.global_prio_queues.clone();
        // split off own priority queue
        let global_prio = global_prios
            .splice(index..=index, iter::empty())
            .next()
            .expect("no priority queue found")
            .fuse();
        // add priority injector queue
        let mut steal_prio = SelectAll::from_iter(global_prios);
        steal_prio.push(EXECUTOR.global_prio_injector.clone());
        let steal_prio = steal_prio.fuse();

        // get normal queues to steal from
        let mut global_normals = EXECUTOR.global_normal_queues.clone();
        // split off own normal queue
        let global_normal = global_normals
            .splice(index..=index, iter::empty())
            .next()
            .expect("no normal queue found")
            .fuse();
        // add normal injector queue
        let mut steal_normal = SelectAll::from_iter(global_normals);
        steal_normal.push(EXECUTOR.global_normal_injector.clone());
        let steal_normal = steal_normal.fuse();

        // build `Queues`
        Self::Async {
            management,
            local_prio,
            global_prio,
            steal_prio,
            local_normal,
            global_normal,
            steal_normal,
        }
    }

    fn init_blocking() -> Self {
        let count = Arc::default();
        let local_prio = Channel::new().fuse();
        let local_normal = Channel::new().fuse();
        let finished = Channel::new().fuse();

        Self::Blocking {
            count,
            local_prio,
            local_normal,
            finished,
        }
    }
}

impl<R> Task<R> {
    fn spawn_internal<F, Q>(task: F, queue: Q) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
        Q: Fn(&Worker) -> &Channel + Send + Sync + 'static,
    {
        #[cfg(feature = "tokio-support")]
        let task = tokio_util::context::TokioContext::new(
            task,
            Worker::with(|worker| worker.tokio.clone()),
        );
        let (runnable, task) = async_task::spawn(task, move |runnable| {
            Worker::with(|worker| Ref::map(worker, &queue).send(Message::Task(runnable)))
        });
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
            if let Runtime::Blocking {
                count, finished, ..
            } = &worker.runtime
            {
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
        let queue = Worker::with(|worker| match &worker.runtime {
            Runtime::Async { local_prio, .. } | Runtime::Blocking { local_prio, .. } => {
                local_prio.get_ref().clone()
            }
            Runtime::Alien { .. } => {
                unreachable!("attempted to `Task::block_on` inside an alien runtime")
            }
        });
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
        Self::spawn_internal(task, |worker| match &worker.runtime {
            Runtime::Async { global_prio, .. } => global_prio.get_ref(),
            // in a blocking worker we want to send away non-local tasks
            // to shut down as soon as possible
            Runtime::Blocking { .. } | Runtime::Alien => &EXECUTOR.global_prio_injector,
        })
    }

    pub fn spawn_local_prio<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |worker| match &worker.runtime {
            Runtime::Async { local_prio, .. } | Runtime::Blocking { local_prio, .. } => {
                local_prio.get_ref().clone()
            }
            Runtime::Alien { .. } => {
                unreachable!("attempted to `Task::spawn_local_prio` inside an alien runtime")
            }
        })
    }

    pub fn spawn<F>(task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send,
    {
        Self::spawn_internal(task, |worker| match &worker.runtime {
            Runtime::Async { global_normal, .. } => global_normal.get_ref(),
            // in a blocking worker we want to send away non-local tasks
            // to shut down as soon as possible
            Runtime::Blocking { .. } | Runtime::Alien => &EXECUTOR.global_normal_injector,
        })
    }

    pub fn spawn_local<F>(task: F) -> Self
    where
        F: Future<Output = R> + 'static,
    {
        Self::spawn_internal_local(task, |worker| match &worker.runtime {
            Runtime::Async { local_normal, .. } | Runtime::Blocking { local_normal, .. } => {
                local_normal.get_ref().clone()
            }
            Runtime::Alien { .. } => {
                unreachable!("attempted to `Task::spawn_local` inside an alien runtime")
            }
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
        // we have to call `spawn` before entering the new thread to return the `Task`
        // but we want the local queue of the new `Worker`, so we register it when we are inside
        let queue: Arc<OnceCell<Channel>> = Arc::default();
        let (runnable, task) = {
            let queue = Arc::clone(&queue);
            async_task::spawn(task, move |task| {
                queue
                    .get()
                    .expect("queue for blocking task not set yet")
                    .send(Message::Task(task))
            })
        };

        // blocking tasks are spawned in a separate start
        Builder::new()
            .name(String::from("blocking"))
            .spawn(move || {
                // register the local queue of the new `Worker`
                queue
                    .set(Worker::with(|worker| {
                        if let Runtime::Blocking { local_prio, .. } = &worker.runtime {
                            local_prio.get_ref().clone()
                        } else {
                            unreachable!("`Worker` isn't a blocking one")
                        }
                    }))
                    .map_err(|_| ())
                    .expect("queue for blocking task already set");
                // run task inside the new thread instead of the spawning one
                runnable.run();
                Worker::start();
            })
            .expect("failed to spawn thread");

        Self(Some(task))
    }
}
