use crate::background_task::BackgroundTask;
use lera::LeraModel;
use std::{
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[lera::state]
pub struct CounterState {
    pub count: i64,
    pub is_auto_incrementing: bool,
    pub auto_increment_interval_ms: u64,
}
impl Default for CounterState {
    fn default() -> Self {
        Self {
            count: 0,
            is_auto_incrementing: true,
            auto_increment_interval_ms: 1000,
        }
    }
}

#[lera::model(state = CounterState)]
pub struct Counter {
    background_task: Mutex<BackgroundTask>,
}

impl Counter {
    fn do_stop_auto_incrementing(&self) {
        println!("Rust: Stopping auto incrementing");
        self.background_task.lock().unwrap().stop();
    }

    fn increment(self: &Arc<Self>) {
        println!("Rust: Incrementing counter");
        self.mutate(|state| {
            state.count += 1;
        });
    }

    fn start_auto_incrementing(self: &Arc<Self>) {
        println!("Rust: Request to start auto incrementing");
        let mut task_guard = self.background_task.lock().unwrap();
        if task_guard.is_running() {
            println!("Rust: Auto-increment task is already running, not starting another");
            return;
        }
        let interval_ms =
            Duration::from_millis(self.access(|state| state.auto_increment_interval_ms));

        // Update state to show auto incrementing is active
        self.mutate(|state| {
            state.is_auto_incrementing = true;
        });

        // Create a weak reference to self for the background task
        let weak_self = Arc::downgrade(self);
        println!(
            "Rust: Starting auto-increment background task with interval {:?}",
            interval_ms
        );
        task_guard.start(interval_ms, move || {
            if let Some(strong_self) = weak_self.upgrade() {
                // Call the existing increment method - no code duplication!
                strong_self.increment();

                // Check if we should continue
                strong_self.access(|state| state.is_auto_incrementing)
            } else {
                println!("Rust: Counter instance has been dropped, stopping auto-increment task");
                false // Counter was dropped, stop the task
            }
        });
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("Rust: Dropping Counter instance, stopping any background task");
        self.do_stop_auto_incrementing();
    }
}

// Exported API
#[lera::api]
impl Counter {
    pub fn increment_button_tapped(self: &Arc<Self>) {
        self.increment();
    }

    pub fn decrement_button_tapped(self: &Arc<Self>) {
        self.mutate(|state| {
            state.count -= 1;
        });
    }

    pub fn reset_button_tapped(self: &Arc<Self>) {
        self.mutate(|state| {
            state.count = 0;
        });
    }

    pub fn start_auto_incrementing_button_tapped(self: &Arc<Self>) {
        self.start_auto_incrementing();
    }

    pub fn stop_auto_incrementing_button_tapped(self: &Arc<Self>) {
        self.mutate(|state| {
            state.is_auto_incrementing = false;
        });
        self.do_stop_auto_incrementing();
    }
}
