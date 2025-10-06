use std::path::Path;
use std::process::Command;

use crate::uniffi_build_swift::cargo_args;
use crate::uniffi_build_swift::commands;

pub struct CargoBuilder {
    command: Command,
}

impl CargoBuilder {
    pub fn new() -> Self {
        Self {
            command: Command::new(commands::CARGO),
        }
    }

    pub fn build_package(mut self, package: &str, manifest_path: &Path, target: &str) -> Self {
        self.command.args([
            cargo_args::BUILD,
            cargo_args::PACKAGE,
            package,
            cargo_args::LIB,
            cargo_args::RELEASE,
            cargo_args::MANIFEST_PATH,
            &manifest_path.to_string_lossy(),
            cargo_args::TARGET,
            target,
        ]);
        self
    }

    pub fn execute(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let output = self.command.output()?;

        if !output.status.success() {
            return Err(format!(
                "Cargo command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(())
    }
}
