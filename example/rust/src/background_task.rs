use std::{
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use tokio::runtime::{Builder, Runtime};

static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    TOKIO_RT.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .thread_name("uniffi-rt")
            .build()
            .expect("Failed to build Tokio runtime")
    })
}

/// Coordinates a cancellable task that is driven from Rust.
///
/// Equality and hashing treat all instances as identical so that models embedding a
/// `BackgroundTask` can implement `Eq` and `Hash` even though the runtime task handle does not
/// participate in those comparisons.
#[derive(Default, Debug)]
pub struct BackgroundTask {
    inner: Mutex<BackgroundTaskInner>,
}

#[derive(Default, Debug)]
struct BackgroundTaskInner {
    handle: Option<tokio::task::JoinHandle<()>>,
}

pub type ShouldContinue = bool;

impl BackgroundTaskInner {
    fn do_start_background_task(
        interval_ms: Duration,
        tick: impl Fn() -> ShouldContinue + Send + 'static,
    ) -> tokio::task::JoinHandle<()> {
        println!("Rust: Starting background task...");
        let runtime = get_runtime();
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(interval_ms);
            // Skip the first tick which fires immediately
            interval.tick().await;

            loop {
                interval.tick().await;
                if !tick() {
                    println!("Rust: Background task stopping as requested");
                    break;
                }
            }
        })
    }

    fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    fn start<F>(&mut self, tick_interval_ms: Duration, tick: F)
    where
        F: Fn() -> ShouldContinue + Send + 'static,
    {
        self.stop();
        self.handle = Some(Self::do_start_background_task(tick_interval_ms, tick));
    }

    fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

// === PUBLIC API ===
impl BackgroundTask {
    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().is_running()
    }

    pub fn start<F>(&self, tick_interval_ms: Duration, tick: F)
    where
        F: Fn() -> ShouldContinue + Send + 'static,
    {
        self.inner
            .lock()
            .expect("BackgroundTask::start failed to acquire lock")
            .start(tick_interval_ms, tick);
    }

    pub fn stop(&self) {
        self.inner
            .lock()
            .expect("BackgroundTask::stop failed to acquire lock")
            .stop();
    }
}

impl PartialEq for BackgroundTask {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for BackgroundTask {}

impl Hash for BackgroundTask {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}
