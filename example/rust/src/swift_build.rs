use clap::Parser;
use lera_uniffi_build::CliSwift;
use lera_uniffi_build::SwiftBuildSettings;

fn main() {
    let cli = CliSwift::parse();
    lera::build_swift(SwiftBuildSettings::from(cli)).expect("Failed to build");
}
