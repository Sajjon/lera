use clap::Parser;

use crate::uniffi_build_android::{AndroidBuildSettings, AndroidTarget};

#[derive(Parser, Debug)]
#[command(name = "android-build")]
#[command(about = "Lera build for Android platforms")]
pub struct CliAndroid {
    /// Path to Android module Kotlin source directory (e.g., "your_package/src/main/kotlin")
    #[arg(long)]
    pub android_sources_dir: Option<String>,

    /// Optional jniLibs directory for copying native artifacts (e.g., "your_package/src/main/jniLibs")
    #[arg(long)]
    pub android_jni_libs_dir: Option<String>,

    /// Targets to build native libraries for (defaults to arm64-v8a and x86_64)
    #[arg(long, value_enum)]
    pub targets: Vec<AndroidTarget>,
}

impl From<CliAndroid> for AndroidBuildSettings {
    fn from(value: CliAndroid) -> Self {
        let CliAndroid {
            android_sources_dir,
            android_jni_libs_dir,
            targets,
        } = value;

        let sources_dir = android_sources_dir
            .expect("--android-sources-dir must be supplied (or provided via defaults)");

        let mut settings = AndroidBuildSettings::new(sources_dir);

        if let Some(dir) = android_jni_libs_dir.filter(|dir| !dir.is_empty()) {
            settings = settings.android_jni_libs_dir(dir);
        }

        if !targets.is_empty() {
            settings = settings.targets(targets);
        }

        settings
    }
}
