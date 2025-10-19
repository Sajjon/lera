mod background_task;
mod counter;
mod manual_only_counter;

pub mod prelude {
    pub use crate::background_task::*;

    pub use lera::{api, model, state, LeraModel};
    pub use std::sync::{Arc, RwLock};
    pub use log::{info, debug, error, warn, trace};
}

uniffi::setup_scaffolding!();
lera::lera_setup_ffi_for_logging!();
