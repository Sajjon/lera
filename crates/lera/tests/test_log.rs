use std::sync::Arc;

pub struct UniFfiTag;

lera::lera_setup_ffi_for_logging!();
use log::debug;

#[test]
fn do_test() {
    struct SwiftLogger;
    impl FfiLogger for SwiftLogger {
        fn log_message(&self, message: String, level: FfiLogLevel) {
            let level = log::Level::from(lera::LogLevel::from(level));
            println!("SwiftLogger: {message}@{level:?}");
        }
    }
    let swift_logger: Arc<SwiftLogger> = Arc::new(SwiftLogger);
    install_logger(swift_logger);
    debug!("Hey");
}
