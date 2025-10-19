use std::sync::{Arc, RwLock};

// Logger struct that implements the `log::Log` trait.
pub struct RustLogger(pub RwLock<Option<Arc<dyn Logger>>>);

pub static RUST_LOGGER: RustLogger = RustLogger(RwLock::new(None));

#[macro_export]
macro_rules! __declare_log_level {
    (
        $(#[$attributes:meta])*
        $name: ident
    ) => {
        $(#[$attributes])*
        pub enum $name {
            /// The "error" level.
            ///
            /// Designates very serious errors.
            // This way these line up with the discriminants for LevelFilter below
            // This works because Rust treats field-less enums the same way as C does:
            // https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-field-less-enumerations
            Error = 1,
            /// The "warn" level.
            ///
            /// Designates hazardous situations.
            Warn,
            /// The "info" level.
            ///
            /// Designates useful information.
            Info,
            /// The "debug" level.
            ///
            /// Designates lower priority information.
            Debug,
            /// The "trace" level.
            ///
            /// Designates very low priority, often extremely verbose, information.
            Trace,
        }
    };
}

__declare_log_level!(
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    LogLevel
);

impl From<log::Level> for LogLevel {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            log::Level::Info => LogLevel::Info,
            log::Level::Debug => LogLevel::Debug,
            log::Level::Trace => LogLevel::Trace,
        }
    }
}

impl From<LogLevel> for log::Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Trace => log::Level::Trace,
        }
    }
}

impl RustLogger {
    fn is_any_logger_installed(&self) -> bool {
        self.0
            .read()
            .ok()
            .and_then(|g| (*g).as_ref().map(|_| ()))
            .is_some()
    }
}
impl log::Log for RustLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        self.is_any_logger_installed()
    }

    fn log(&self, record: &log::Record<'_>) {
        let maybe_logger = &*self.0.read().expect("RUST_LOGGER poisoned");
        if let Some(foreign_logger) = maybe_logger {
            foreign_logger.log_message(record.args().to_string(), LogLevel::from(record.level()));
        }
    }

    fn flush(&self) {}
}

#[macro_export]
macro_rules! __declare_logger {
    (
        $(#[$attributes:meta])*
        $name: ident,
        $level_ty: ty
    ) => {
        $(#[$attributes])*
        pub trait $name: Sync + Send {
            fn log_message(&self, message: String, level: $level_ty);
        }
    };
}

__declare_logger!(Logger, LogLevel);

#[macro_export]
macro_rules! lera_setup_ffi_for_logging {
    () => {
        ::lera::__inner_lera_setup_ffi_for_logging!(FfiLogger);
    };
}

#[macro_export]
macro_rules! __inner_lera_setup_ffi_for_logging {
    ($trait_name: ident) => {
        ::lera::__declare_log_level!(
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, uniffi::Enum)]
            FfiLogLevel
        );

        impl From<lera::LogLevel> for FfiLogLevel {
            fn from(value: lera::LogLevel) -> Self {
                match value {
                    lera::LogLevel::Error => FfiLogLevel::Error,
                    lera::LogLevel::Warn => FfiLogLevel::Warn,
                    lera::LogLevel::Info => FfiLogLevel::Info,
                    lera::LogLevel::Debug => FfiLogLevel::Debug,
                    lera::LogLevel::Trace => FfiLogLevel::Trace,
                }
            }
        }


        impl From<FfiLogLevel> for lera::LogLevel {
            fn from(value: FfiLogLevel) -> Self {
                match value {
                    FfiLogLevel::Error => lera::LogLevel::Error,
                    FfiLogLevel::Warn => lera::LogLevel::Warn,
                    FfiLogLevel::Info => lera::LogLevel::Info,
                    FfiLogLevel::Debug => lera::LogLevel::Debug,
                    FfiLogLevel::Trace => lera::LogLevel::Trace,
                }
            }
        }


        ::lera::__declare_logger!(
            /// Logger trait that the foreign code implements
            #[uniffi::export(with_foreign)]
            $trait_name,
            FfiLogLevel
        );

        impl ::lera::Logger for dyn $trait_name {
            fn log_message(&self, message: String, level: lera::LogLevel) {
                $trait_name::log_message(self, message, FfiLogLevel::from(level))
            }
        }

        fn init() {
            static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

            ONCE.get_or_init(|| {
                if let Err(e) = log::set_logger(&lera::RUST_LOGGER) {
                    log::warn!("Logger already set or failed to install logger: {}", e);
                }
                log::set_max_level(log::LevelFilter::Trace);
            });
        }

        #[uniffi::export]
        pub fn rust_diagnostics_log_at_all_levels() {
            log::trace!("Trace");
            log::debug!("Debug");
            log::info!("Info");
            log::warn!("Warn");
            log::error!("Error");
        }

        #[uniffi::export]
        pub fn install_logger(logger: std::sync::Arc<dyn $trait_name>) {
            init();
            struct Bridge {
                inner: std::sync::Arc<dyn $trait_name>,
            }
            impl lera::Logger for Bridge {
                fn log_message(&self, message: String, level: lera::LogLevel) {
                    self.inner.log_message(message, FfiLogLevel::from(level))
                }
            }
            let bridged: std::sync::Arc<dyn ::lera::Logger> = std::sync::Arc::new(Bridge { inner: logger });
            *lera::RUST_LOGGER.0.write().expect("RUST_LOGGER poisoned") = Some(bridged);
        }
    };
}
