use anyhow::{anyhow, Result};
use async_task::{JoinHandle, Task};
use std::future::Future;
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub type ScheduleFunc = Box<dyn Fn(Task<()>) + Send + Sync + 'static>;

fn no_schedule_configured(_: Task<()>) {
    panic!("no scheduler has been configured");
}

lazy_static::lazy_static! {
    static ref ON_MAIN_THREAD: Mutex<ScheduleFunc> = Mutex::new(Box::new(no_schedule_configured));
    static ref ON_MAIN_THREAD_LOW_PRI: Mutex<ScheduleFunc> = Mutex::new(Box::new(no_schedule_configured));
    static ref TOKIO: tokio::runtime::Handle = start_tokio();
}

/// Set callbacks for scheduling normal and low priority futures.
/// Why this and not "just tokio"?  In a GUI application there is typically
/// a special GUI processing loop that may need to run on the "main thread",
/// so we can't just run a tokio/mio loop in that context.
/// This particular crate has no real knowledge of how that plumbing works,
/// it just provides the abstraction for scheduling the work.
/// This function allows the embedding application to set that up.
pub fn set_schedulers(main: ScheduleFunc, low_pri: ScheduleFunc) {
    *ON_MAIN_THREAD.lock().unwrap() = Box::new(main);
    *ON_MAIN_THREAD_LOW_PRI.lock().unwrap() = Box::new(low_pri);
}

/// Spawn the tokio runtime and run it on a secondary thread.
/// We can't run it on the main thread for the reasons mentioned above.
/// This is called implicitly when the TOKIO global is accessed by the
/// `tokio_spawn` function below.
fn start_tokio() -> tokio::runtime::Handle {
    let mut runtime = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .core_threads(1)
        .build()
        .expect("failed to initialize tokio runtime");
    let handle = runtime.handle().clone();

    // Run the runtime in another thread.
    // I'm not sure that it is strictly needed, or whether we can just
    // keep it alive without actively polling anything.
    std::thread::spawn(move || {
        // A future that never completes
        struct Never {}
        impl Future for Never {
            type Output = ();
            fn poll(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context,
            ) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        // manage the runtime forever
        runtime.block_on(Never {});
    });
    handle
}

/// Spawn a future into the tokio runtime, spawning the tokio runtime
/// if it hasn't already been started up.  The tokio runtime (in the
/// context of this crate) is intended primarily for scheduling network
/// IO.  Most futures should be spawned via the other functions provided
/// by this module.
pub fn tokio_spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TOKIO.spawn(future)
}

/// Spawn a new thread to execute the provided function.
/// Returns a JoinHandle that implements the Future trait
/// and that can be used to await and yield the return value
/// from the thread.
/// Can be called from any thread.
pub fn spawn_into_new_thread<F, T>(f: F) -> JoinHandle<Result<T>, ()>
where
    F: FnOnce() -> Result<T>,
    F: Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = sync_channel(1);

    // Holds the waker that may later observe
    // during the Future::poll call.
    struct WakerHolder {
        waker: Mutex<Option<Waker>>,
    }

    let holder = Arc::new(WakerHolder {
        waker: Mutex::new(None),
    });

    let thread_waker = Arc::clone(&holder);
    std::thread::spawn(move || {
        // Run the thread
        let res = f();
        // Pass the result back
        tx.send(res).unwrap();
        // If someone polled the thread before we got here,
        // they will have populated the waker; extract it
        // and wake up the scheduler so that it will poll
        // the result again.
        let mut waker = thread_waker.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    });

    struct PendingResult<T> {
        rx: Receiver<Result<T>>,
        holder: Arc<WakerHolder>,
    }

    impl<T> std::future::Future for PendingResult<T> {
        type Output = Result<T>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
            match self.rx.try_recv() {
                Ok(res) => Poll::Ready(res),
                Err(TryRecvError::Empty) => {
                    let mut waker = self.holder.waker.lock().unwrap();
                    waker.replace(cx.waker().clone());
                    Poll::Pending
                }
                Err(TryRecvError::Disconnected) => {
                    Poll::Ready(Err(anyhow!("thread terminated without providing a result")))
                }
            }
        }
    }

    spawn_into_main_thread(PendingResult { rx, holder })
}

/// Spawn a future into the main thread; it will be polled in the
/// main thread.
/// This function can be called from any thread.
/// If you are on the main thread already, consider using
/// spawn() instead to lift the `Send` requirement.
pub fn spawn_into_main_thread<F, R>(future: F) -> JoinHandle<R, ()>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn(future, |task| ON_MAIN_THREAD.lock().unwrap()(task), ());
    task.schedule();
    handle
}

/// Spawn a future into the main thread; it will be polled in
/// the main thread in the low priority queue--all other normal
/// priority items will be drained before considering low priority
/// spawns.
/// If you are on the main thread already, consider using `spawn_with_low_priority`
/// instead to lift the `Send` requirement.
pub fn spawn_into_main_thread_with_low_priority<F, R>(future: F) -> JoinHandle<R, ()>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn(
        future,
        |task| ON_MAIN_THREAD_LOW_PRI.lock().unwrap()(task),
        (),
    );
    task.schedule();
    handle
}

/// Spawn a future with normal priority.
pub fn spawn<F, R>(future: F) -> JoinHandle<R, ()>
where
    F: Future<Output = R> + 'static,
    R: 'static,
{
    let (task, handle) =
        async_task::spawn_local(future, |task| ON_MAIN_THREAD.lock().unwrap()(task), ());
    task.schedule();
    handle
}

/// Spawn a future with low priority; it will be polled only after
/// all other normal priority items are processed.
pub fn spawn_with_low_priority<F, R>(future: F) -> JoinHandle<R, ()>
where
    F: Future<Output = R> + 'static,
    R: 'static,
{
    let (task, handle) = async_task::spawn_local(
        future,
        |task| ON_MAIN_THREAD_LOW_PRI.lock().unwrap()(task),
        (),
    );
    task.schedule();
    handle
}

/// Block the current thread until the passed future completes.
pub use async_std::task::block_on;

pub async fn join_handle_result<T>(handle: JoinHandle<anyhow::Result<T>, ()>) -> anyhow::Result<T> {
    handle
        .await
        .ok_or_else(|| anyhow::anyhow!("task was cancelled or panicked"))?
}
