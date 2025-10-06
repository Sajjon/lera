use clap::Parser;
use lera_uniffi_build::CliAndroid;

fn main() {
    let cli = CliAndroid::parse();
    lera::build_android(cli.into()).expect("Failed to build Android bindings");
}
