mod cargo_utils;
mod uniffi_build_android;
mod uniffi_build_android_cli;
mod uniffi_build_swift;
mod uniffi_build_swift_cli;

pub mod prelude {
    pub use crate::uniffi_build_android::{
        AndroidBuildOutcome, AndroidBuildSettings, AndroidTarget, build_android,
    };
    pub use crate::uniffi_build_android_cli::CliAndroid;
    pub use crate::uniffi_build_swift::{BuildOutcome, SwiftBuildSettings, build_swift};
    pub use crate::uniffi_build_swift_cli::CliSwift;
}

pub use prelude::*;
