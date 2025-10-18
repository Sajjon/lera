mod bindgen;
pub use bindgen::{build_android, build_swift};
pub use lera_macros::{api, default_params, model, state};
pub use lera_uniffi_build::{AndroidBuildSettings, AndroidTarget, SwiftBuildSettings};
pub use samples_core::Samples;
use std::sync::{Arc, RwLock};

pub mod fmt_utils {
    use core::fmt;

    trait DisplayOrDebug<'a, T: ?Sized> {
        fn fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    impl<'a, T> DisplayOrDebug<'a, T> for &'a T
    where
        T: fmt::Display + ?Sized,
    {
        fn fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }

    impl<'a, T> DisplayOrDebug<'a, T> for &'a &'a T
    where
        T: fmt::Debug + ?Sized,
    {
        fn fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(*self, f)
        }
    }

    pub fn fmt_model_state<T>(state: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result
    where
        T: fmt::Debug,
    {
        DisplayOrDebug::fmt(&state, f)
    }
}

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
    type NavigatorDeps;

    fn new(
        state: Self::State,
        listener: Self::Listener,
        navigator_deps: Self::NavigatorDeps,
    ) -> Arc<Self>
    where
        Self: Sized;

    fn get_state_change_listener(&self) -> &Self::Listener;
    fn get_state_guard(&self) -> &Arc<RwLock<Self::State>>;

    fn access<R: Clone>(&self, access: impl FnOnce(Self::State) -> R) -> R {
        access(
            self.get_state_guard()
                .read()
                .expect("LeraModel::access failed to acquire read lock")
                .clone(),
        )
    }

    fn mutate<R>(&self, mutate: impl FnOnce(&mut Self::State) -> R) -> R {
        let (out, should_notify, new_state) = {
            let mut write_guard = self
                .get_state_guard()
                .write()
                .expect("LeraModel::mutate failed to acquire write lock");
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
