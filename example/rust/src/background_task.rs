use std::{sync::OnceLock, time::Duration};

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

#[derive(Default)]
pub struct BackgroundTask {
    handle: Option<tokio::task::JoinHandle<()>>,
}

pub type ShouldContinue = bool;

// === PRIVATE API ===
impl BackgroundTask {
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

    pub fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

// === PUBLIC API ===
impl BackgroundTask {

    pub fn start<F>(&mut self, tick_interval_ms: Duration, tick: F)
    where
        F: Fn() -> ShouldContinue + Send + 'static,
    {
        self.stop();
        self.handle = Some(Self::do_start_background_task(tick_interval_ms, tick));
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}
