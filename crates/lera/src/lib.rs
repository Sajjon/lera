mod bindgen;
pub use bindgen::{build_android, build_swift};
pub use lera_macros::{api, default_params, model, state};
pub use lera_uniffi_build::{AndroidBuildSettings, AndroidTarget, SwiftBuildSettings};

use std::sync::{Arc, RwLock};

pub trait ModelState: std::fmt::Debug + Clone + PartialEq + Default {}
impl<T: std::fmt::Debug + Clone + PartialEq + Default> ModelState for T {}

/// Macro to generate the boilerplate implementation to bridge UniFFI traits to StateChangeListener
#[macro_export]
macro_rules! impl_state_change_listener_bridge {
    ($trait_name:ident, $state_type:ty) => {
        // Only implement for the trait object to avoid conflicts
        impl ::lera::StateChangeListener for dyn $trait_name {
            type State = $state_type;
            fn on_state_change(&self, new_state: Self::State) {
                $trait_name::on_state_change(self, new_state)
            }
        }
    };
}

pub trait StateChangeListener: Send + Sync + 'static {
    type State: ModelState;
    fn on_state_change(&self, new_state: Self::State);
}

impl<T: StateChangeListener + ?Sized> StateChangeListener for Arc<T> {
    type State = T::State;
    fn on_state_change(&self, new_state: Self::State) {
        (**self).on_state_change(new_state)
    }
}

pub trait LeraModel {
    type State: ModelState;
    type Listener: StateChangeListener<State = Self::State>;

    fn new(state: Self::State, listener: Self::Listener) -> Arc<Self>
    where
        Self: Sized;

    fn get_state_change_listener(&self) -> &Self::Listener;
    fn get_state_guard(&self) -> &Arc<RwLock<Self::State>>;

    fn access<R: Clone>(&self, access: impl FnOnce(Self::State) -> R) -> R {
        access(self.get_state_guard().try_read().unwrap().clone())
    }

    fn mutate<R>(&self, mutate: impl FnOnce(&mut Self::State) -> R) -> R {
        let (out, should_notify, new_state) = {
            let mut write_guard = self.get_state_guard().try_write().unwrap();
            let prev_state = write_guard.clone();
            let out = mutate(&mut write_guard);
            let new_state = write_guard.clone();
            let should_notify = new_state != prev_state;
            (out, should_notify, new_state)
        };

        if should_notify {
            self.notify_state_change(new_state);
        }
        out
    }

    fn notify_state_change(&self, new_state: Self::State) {
        println!("Rust: Notifying listener of state change: {:?}", new_state);
        self.get_state_change_listener().on_state_change(new_state);
    }
}
