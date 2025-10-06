use clap::Parser;

use crate::uniffi_build_swift::SwiftBuildSettings;

#[derive(Parser, Debug)]
#[command(name = "apple-build")]
#[command(about = "Lera build for Apple platforms")]
pub struct CliSwift {
    /// Path to Apple project Swift source directory (e.g., "apple/Sources/UniFFI/")
    #[arg(long)]
    pub apple_sources_dir: String,

    /// Build for macOS only (skip iOS targets)
    #[arg(long)]
    pub maconly: bool,

    /// Release tag for Package.swift
    #[arg(long)]
    pub release_tag: Option<String>,
}

impl From<CliSwift> for SwiftBuildSettings {
    fn from(cli: CliSwift) -> Self {
        let mut settings = SwiftBuildSettings::new(cli.apple_sources_dir).maconly(cli.maconly);

        if let Some(tag) = cli.release_tag {
            settings = settings.release_tag(tag);
        }

        settings
    }
}
