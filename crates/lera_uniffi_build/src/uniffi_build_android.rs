use std::fs;
use std::path::{Path, PathBuf};

use camino::Utf8PathBuf;
use clap::ValueEnum;
use uniffi_bindgen::bindings::KotlinBindingGenerator;

use crate::cargo_utils::CargoBuilder;
use crate::uniffi_build_swift::{env_vars, extensions, messages, paths, targets};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AndroidTarget {
    #[value(alias = "arm64-v8a")]
    Arm64V8a,
    #[value(alias = "armeabi-v7a")]
    ArmeabiV7a,
    #[value(alias = "x86")]
    X86,
    #[value(alias = "x86_64")]
    X86_64,
}

impl AndroidTarget {
    fn triple(&self) -> &'static str {
        match self {
            AndroidTarget::Arm64V8a => "aarch64-linux-android",
            AndroidTarget::ArmeabiV7a => "armv7-linux-androideabi",
            AndroidTarget::X86 => "i686-linux-android",
            AndroidTarget::X86_64 => "x86_64-linux-android",
        }
    }

    fn abi_dir(&self) -> &'static str {
        match self {
            AndroidTarget::Arm64V8a => "arm64-v8a",
            AndroidTarget::ArmeabiV7a => "armeabi-v7a",
            AndroidTarget::X86 => "x86",
            AndroidTarget::X86_64 => "x86_64",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AndroidBuildSettings {
    pub android_sources_dir: String,
    pub android_jni_libs_dir: Option<String>,
    pub targets: Vec<AndroidTarget>,
}

impl AndroidBuildSettings {
    pub fn new(android_sources_dir: impl Into<String>) -> Self {
        Self {
            android_sources_dir: android_sources_dir.into(),
            android_jni_libs_dir: None,
            targets: vec![AndroidTarget::Arm64V8a, AndroidTarget::X86_64],
        }
    }

    pub fn android_jni_libs_dir(mut self, dir: impl Into<String>) -> Self {
        self.android_jni_libs_dir = Some(dir.into());
        self
    }

    pub fn targets(mut self, targets: Vec<AndroidTarget>) -> Self {
        self.targets = targets;
        self
    }
}

#[derive(Debug, Clone)]
pub struct AndroidBuildOutcome {
    pub kotlin_file_path: PathBuf,
    pub path_to_crate: PathBuf,
    pub jni_lib_paths: Vec<PathBuf>,
}

pub fn build_android(
    settings: AndroidBuildSettings,
) -> Result<AndroidBuildOutcome, Box<dyn std::error::Error>> {
    println!("🤖 Start of Android build");
    let package_name =
        std::env::var(env_vars::CARGO_PKG_NAME).expect("CARGO_PKG_NAME env var should be set");
    let path_to_crate: PathBuf = std::env::var(env_vars::CARGO_MANIFEST_DIR)
        .expect("CARGO_MANIFEST_DIR env var should be set")
        .into();

    println!(
        "{} lera_android_build::build_android - settings {:?}",
        messages::BUILD_START,
        settings
    );

    let host_target = std::env::var("HOST").unwrap_or_else(|_| targets::MACOS.to_string());
    build_target(&package_name, &path_to_crate, &host_target)?;

    let dylib_path = dynamic_lib_path(
        &path_to_crate,
        &host_target,
        &package_name,
        extensions::DYNAMIC_LIB,
    );

    let sources_dir = resolve_relative_dir(&path_to_crate, &settings.android_sources_dir)?;
    fs::create_dir_all(&sources_dir)?;

    let kotlin_file_path = generate_kotlin_bindings(&dylib_path, &sources_dir)?;

    let mut jni_lib_paths = Vec::new();
    if let Some(jni_dir_rel) = &settings.android_jni_libs_dir {
        let jni_dir = resolve_relative_dir(&path_to_crate, jni_dir_rel)?;
        for target in &settings.targets {
            build_target(&package_name, &path_to_crate, target.triple())?;
            let artifact = dynamic_lib_path(&path_to_crate, target.triple(), &package_name, "so");
            let dest_dir = jni_dir.join(target.abi_dir());
            fs::create_dir_all(&dest_dir)?;
            let dest = dest_dir.join(
                artifact
                    .file_name()
                    .ok_or("Failed to derive artifact file name")?,
            );
            fs::copy(&artifact, &dest).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to copy {:?} to {:?}. Ensure Android NDK toolchain is installed and cargo target configuration exists. {}",
                        artifact, dest, error
                    ),
                )
            })?;
            jni_lib_paths.push(dest);
        }
    }

    println!("{} Android build completed", messages::SUCCESS);

    Ok(AndroidBuildOutcome {
        kotlin_file_path,
        path_to_crate,
        jni_lib_paths,
    })
}

fn build_target(
    package_name: &str,
    crate_path: &Path,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = crate_path.join(paths::CARGO_TOML);
    CargoBuilder::new()
        .build_package(package_name, &cargo_toml, target)
        .execute()
        .map_err(|e| {
            format!(
                "Failed to build target {}. Ensure required toolchains are installed: {}",
                target, e
            )
            .into()
        })
}

fn dynamic_lib_path(crate_path: &Path, target: &str, package: &str, extension: &str) -> PathBuf {
    crate_path.join(paths::RUST_BUILD_DIR).join(format!(
        "{}/{}/lib{}.{}",
        target,
        paths::RELEASE_SUBDIR,
        package,
        extension
    ))
}

fn resolve_relative_dir(
    crate_path: &Path,
    relative: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parent = crate_path
        .parent()
        .ok_or("Cannot find parent directory of crate")?;
    Ok(parent.join(relative))
}

fn generate_kotlin_bindings(
    dylib_path: &Path,
    out_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_supplier = uniffi_bindgen::EmptyCrateConfigSupplier;

    let dylib_utf8 = Utf8PathBuf::from_path_buf(dylib_path.to_path_buf()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Non UTF-8 path for dynamic library: {:?}", dylib_path),
        )
    })?;
    let out_dir_utf8 = Utf8PathBuf::from_path_buf(out_dir.to_path_buf()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Non UTF-8 path for Kotlin output directory: {:?}", out_dir),
        )
    })?;

    let components = uniffi_bindgen::library_mode::generate_bindings(
        &dylib_utf8,
        None,
        &KotlinBindingGenerator,
        &config_supplier,
        None,
        &out_dir_utf8,
        false,
    )?;

    let component = components
        .first()
        .ok_or("No UniFFI components discovered when generating Kotlin bindings")?;
    let package_path: PathBuf = component.config.package_name().split('.').collect();
    let kotlin_file = out_dir
        .join(package_path)
        .join(format!("{}.kt", component.ci.namespace()));

    if !kotlin_file.exists() {
        return Err(format!(
            "Expected Kotlin bindings at {:?} but file was not generated",
            kotlin_file
        )
        .into());
    }

    println!("{} Generated Kotlin bindings", messages::FFI_GEN);

    Ok(kotlin_file)
}
