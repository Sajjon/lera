use crate::background_task::BackgroundTask;
use lera::LeraModel;
use samples_derive::Samples;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

/// A non zero interval in milliseconds
#[derive(Clone, Debug, PartialEq, Samples, Eq, Hash, uniffi::Record, derive_more::Deref)]
pub struct Interval {
    #[samples([500, 1000] -> const_try_from)]
    ms: u64,
}

impl Interval {
    pub const fn const_try_from(value: u64) -> Result<Self, &'static str> {
        if value == 0 {
            Err("Interval must be non-zero")
        } else {
            Ok(Interval { ms: value })
        }
    }
}

impl From<Interval> for Duration {
    fn from(interval: Interval) -> Self {
        Duration::from_millis(interval.ms)
    }
}
impl TryFrom<u64> for Interval {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::const_try_from(value).map_err(|err| err.to_string())
    }
}
impl Default for Interval {
    fn default() -> Self {
        Self::try_from(1000).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[lera::state]
pub struct CounterState {
    pub count: i64,
    pub is_auto_incrementing: bool,
    pub auto_increment_interval_ms: Interval,
}
impl Default for CounterState {
    fn default() -> Self {
        Self {
            count: 0,
            is_auto_incrementing: true,
            auto_increment_interval_ms: Interval::default(),
        }
    }
}

#[lera::model(state = CounterState)]
pub struct Counter {
    background_task: BackgroundTask,
}

impl Counter {
    fn do_stop_auto_incrementing(&self) {
        println!("Rust: Stopping auto incrementing");
        self.background_task.stop();
    }

    fn increment(self: &Arc<Self>) {
        println!("Rust: Incrementing counter");
        self.mutate(|state| {
            state.count += 1;
        });
    }

    fn start_auto_incrementing(self: &Arc<Self>) {
        println!("Rust: Request to start auto incrementing");
        if self.background_task.is_running() {
            println!("Rust: Auto-increment task is already running, not starting another");
            return;
        }
        let interval_ms = Duration::from(self.access(|state| state.auto_increment_interval_ms));

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
        self.background_task.start(interval_ms, move || {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_equal_when_states_match() {
        let state = CounterState {
            count: 0,
            is_auto_incrementing: false,
            auto_increment_interval_ms: Interval::try_from(1).unwrap(),
        };
        let a = Counter::without_listener(state.clone(), BackgroundTask::default());
        let b = Counter::without_listener(state, BackgroundTask::default());
        assert_eq!(a, b);
    }

    #[test]
    fn arc_counters_equal_when_states_match() {
        let state = CounterState {
            count: 1,
            is_auto_incrementing: false,
            auto_increment_interval_ms: Interval::try_from(5).unwrap(),
        };
        let listener: Arc<dyn CounterStateChangeListener> =
            Arc::new(super::CounterNoopListener::default());
        let other_listener: Arc<dyn CounterStateChangeListener> =
            Arc::new(super::CounterNoopListener::default());
        let a = Counter::new(state.clone(), listener);
        let b = Counter::new(state, other_listener);
        assert_eq!(a, b);
    }

    #[test]
    fn debug_formats_state() {
        let counter = Counter::without_listener(CounterState::default(), BackgroundTask::default());
        let output = format!("{:?}", counter);
        assert!(output.contains("CounterState"));
    }

    #[test]
    fn display_formats_state() {
        let counter = Counter::without_listener(
            CounterState {
                count: 42,
                is_auto_incrementing: false,
                auto_increment_interval_ms: Interval::try_from(100).unwrap(),
            },
            BackgroundTask::default(),
        );
        let output = format!("{}", counter);
        assert!(output.contains("42"));
    }

    use samples_core::Samples;
    
    #[test]
    fn try_from_syntax_is_possible() {
        let samples: Vec<Interval> = Interval::sample_vec();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], Interval::const_try_from(500).unwrap());
        assert_eq!(samples[1], Interval::const_try_from(1000).unwrap());
    }
}
