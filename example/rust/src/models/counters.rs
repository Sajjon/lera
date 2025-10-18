use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[lera::state]
pub struct CountersState {
    pub counters: Vec<Arc<Counter>>,
}

#[lera::model(state = CountersState, navigating)]
pub struct Counters {}

// Exported API
#[lera::api(navigating)]
impl Counters {
    pub fn counter_tapped(&self, index: u32) {
        let index = index as usize;
        println!("Counter tapped at index: {}", index);
        self.access(|state| {
            let counter = state.counters.get(index).expect("Not found");
            self.navigator.push_screen(counter.clone().into())
        });
    }

    pub fn new_counter(&self, listener: Arc<dyn CounterStateChangeListener>) {
        println!("Creating new counter");
        let state = CounterState::default();
        let counter = Counter::new(state, listener, ());
        self.mutate(|state| {
            state.counters.push(counter);
        });
    }
}
