use crate::prelude::*;

#[derive(uniffi::Enum, Clone, PartialEq, Eq, Hash)]
#[uniffi::export(Hash, Eq)]
pub enum Screen {
    Counter { model: Arc<Counter> },
    ManualOnlyCounter { model: Arc<ManualOnlyCounter> },
}

pub trait Navigatable: lera::LeraModel + Into<Screen> {}

impl From<Arc<Counter>> for Screen {
    fn from(model: Arc<Counter>) -> Self {
        Self::Counter { model }
    }
}
impl From<Arc<ManualOnlyCounter>> for Screen {
    fn from(model: Arc<ManualOnlyCounter>) -> Self {
        Self::ManualOnlyCounter { model }
    }
}

/// FFI side listening to changes from Rust
#[uniffi::export(with_foreign)]
pub trait ListenerOfNavigationChangesMadeByRust: Send + Sync {
    fn path_changed_in_rust(&self, path: Vec<Screen>);
}

#[derive(Default)]
pub struct AppScreenPath {
    screen_stack: RwLock<Vec<Screen>>,
}
impl AppScreenPath {
    fn mutate(&self, mutate: impl FnOnce(&mut Vec<Screen>)) {
        let mut stack = self
            .screen_stack
            .write()
            .expect("Should be able to acquire write lock for screen_stack in AppScreenPath");
        mutate(&mut stack)
    }

    pub fn push_screen_and_notify(&self, screen: Screen, on_change: impl FnOnce(Vec<Screen>)) {
        self.mutate(|stack| {
            stack.push(screen);
            on_change(stack.to_vec());
        })
    }
    pub fn pop_without_notify(&self) {
        self.mutate(|stack| {
            let _ = stack.pop();
        })
    }
}

#[derive(uniffi::Object)]
pub struct Navigator {
    path: AppScreenPath,
    listener_on_ffi_side: Arc<dyn ListenerOfNavigationChangesMadeByRust>,
}

impl RustNavigation for Navigator {
    fn push_screen(&self, screen: Screen) {
        self.path.push_screen_and_notify(screen, |changed| {
            self.listener_on_ffi_side.path_changed_in_rust(changed)
        })
    }

    fn pop(&self) {
        self.path.pop_without_notify()
    }
}

pub trait RustNavigation {
    fn pop(&self);
    fn push_screen(&self, screen: Screen);
}

#[uniffi::export]
impl Navigator {
    #[uniffi::constructor]
    pub fn new(listener_on_ffi_side: Arc<dyn ListenerOfNavigationChangesMadeByRust>) -> Self {
        Self {
            listener_on_ffi_side,
            path: AppScreenPath::default(),
        }
    }

    pub fn navigation_popped(&self) {
        self.pop()
    }
}
