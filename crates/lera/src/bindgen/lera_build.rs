use lera_uniffi_build::{AndroidBuildSettings, SwiftBuildSettings};

use crate::bindgen::post_process;

pub fn build_swift(settings: SwiftBuildSettings) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = lera_uniffi_build::build_swift(settings)?;
    post_process::post_process_swift(&outcome.swift_file_path, &outcome.path_to_crate);
    Ok(())
}

pub fn build_android(settings: AndroidBuildSettings) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = lera_uniffi_build::build_android(settings)?;
    post_process::post_process_kotlin(&outcome.kotlin_file_path, &outcome.path_to_crate);
    Ok(())
}
