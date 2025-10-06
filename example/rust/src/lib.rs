mod background_task;
mod counter;
mod manual_only_counter;

pub mod prelude {
    pub use crate::background_task::*;

    pub use lera::{LeraModel, api, model, state};
    pub use std::sync::{Arc, RwLock};
}

uniffi::setup_scaffolding!();
