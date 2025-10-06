use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cargo_utils::CargoBuilder;

// ==================== CONSTANTS ====================

/// Apple platform target architectures
pub(crate) mod targets {
    /// macOS (Apple Silicon)
    pub const MACOS: &str = "aarch64-apple-darwin";
    /// iOS device (ARM64)
    pub const IOS: &str = "aarch64-apple-ios";
    /// iOS Simulator (Apple Silicon)
    pub const IOS_SIM: &str = "aarch64-apple-ios-sim";
}

/// Build-related directory and file names
pub(crate) mod paths {
    /// Rust/Cargo build output directory name
    pub const RUST_BUILD_DIR: &str = "target";
    /// UniFFI xcframework staging subdirectory name
    pub const XCFRAMEWORK_STAGING_SUBDIR: &str = "xcframework-staging";
    /// Swift build subdirectory name
    pub const SWIFT_SUBDIR: &str = "swift";
    /// Release subdirectory name
    pub const RELEASE_SUBDIR: &str = "release";
    /// Package.swift file name
    pub const PACKAGE_SWIFT: &str = "Package.swift";
    /// Cargo.toml file name
    pub const CARGO_TOML: &str = "Cargo.toml";
    /// Module map file name
    pub const MODULE_MAP: &str = "module.modulemap";
}

/// File extensions
pub(crate) mod extensions {
    /// Swift source file extension
    pub const SWIFT: &str = "swift";
    /// Static library extension
    pub const STATIC_LIB: &str = "a";
    /// Dynamic library extension
    pub const DYNAMIC_LIB: &str = "dylib";
}

/// Cargo command arguments
pub(crate) mod cargo_args {
    /// Build command
    pub const BUILD: &str = "build";
    /// Package flag
    pub const PACKAGE: &str = "-p";
    /// Library target type
    pub const LIB: &str = "--lib";
    /// Release mode
    pub const RELEASE: &str = "--release";
    /// Manifest path flag
    pub const MANIFEST_PATH: &str = "--manifest-path";
    /// Target flag
    pub const TARGET: &str = "--target";
}

/// Build status messages
pub(crate) mod messages {
    /// Build start emoji
    pub const BUILD_START: &str = "🧱";
    /// Package build emoji
    pub const PACKAGE_BUILD: &str = "📦";
    /// FFI generation emoji
    pub const FFI_GEN: &str = "🔮";
    /// Success emoji
    pub const SUCCESS: &str = "✅";
    /// Celebration emoji
    #[allow(dead_code)]
    pub const CELEBRATION: &str = "🎉";
    /// Error emoji
    #[allow(dead_code)]
    pub const ERROR: &str = "❌";

    /// Build completion message
    #[allow(dead_code)]
    pub const BUILD_COMPLETE: &str = "Build completed successfully!";
}

/// Environment variable names
pub(crate) mod env_vars {
    /// Cargo package name
    pub const CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
    /// Cargo manifest directory
    pub const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
}

/// Command names
pub(crate) mod commands {
    /// Cargo command
    pub const CARGO: &str = "cargo";
    /// Xcodebuild command
    pub const XCODEBUILD: &str = "xcodebuild";
    /// ZIP command
    pub const ZIP: &str = "zip";
    /// Swift command
    pub const SWIFT: &str = "swift";
}

/// Xcodebuild arguments
mod xcode_args {
    /// Create XCFramework flag
    pub const CREATE_XCFRAMEWORK: &str = "-create-xcframework";
    /// Library flag
    pub const LIBRARY: &str = "-library";
    /// Headers flag
    pub const HEADERS: &str = "-headers";
    /// Output flag
    pub const OUTPUT: &str = "-output";
}

/// Swift Package Manager arguments
mod swift_args {
    /// Package command
    pub const PACKAGE: &str = "package";
    /// Compute checksum subcommand
    pub const COMPUTE_CHECKSUM: &str = "compute-checksum";
}

/// ZIP command arguments
mod zip_args {
    /// Recursive flag
    pub const RECURSIVE: &str = "-r";
}

// ==================== TYPES ====================

/// Build configuration settings
#[derive(Clone, Debug)]
pub struct SwiftBuildSettings {
    /// Build only for macOS (skip iOS targets)
    pub maconly: bool,
    /// Release tag for Package.swift (if provided, enables release mode)
    pub release_tag: Option<String>,
    /// Path to Apple project Swift source directory (e.g., "apple/Sources/UniFFI/")
    pub apple_sources_dir: String,
}

impl SwiftBuildSettings {
    /// Create a new BuildSettings with required apple_sources_dir
    pub fn new(apple_sources_dir: impl Into<String>) -> Self {
        Self {
            maconly: true, // Default to macOS only for faster dev builds
            release_tag: None,
            apple_sources_dir: apple_sources_dir.into(),
        }
    }

    /// Set maconly flag (chainable)
    pub fn maconly(mut self, maconly: bool) -> Self {
        self.maconly = maconly;
        self
    }

    /// Set release tag (chainable)
    pub fn release_tag(mut self, tag: impl Into<String>) -> Self {
        self.release_tag = Some(tag.into());
        self
    }
}

/// Internal build configuration
#[derive(Clone, Debug)]
struct BuildConfig {
    package_name: String,
    path_to_crate: PathBuf,
    settings: SwiftBuildSettings,
}

impl BuildConfig {
    /// Whether this is a release build
    fn is_release(&self) -> bool {
        self.settings.release_tag.is_some()
    }

    /// Get the target architecture for dylib path
    fn dylib_target(&self) -> &'static str {
        if self.settings.maconly {
            targets::MACOS
        } else {
            targets::IOS
        }
    }

    /// Get FFI module name (package name + "FFI" suffix)
    fn module_name(&self) -> String {
        format!("{}FFI", self.package_name)
    }

    /// Get XCFramework file name
    fn xcframework_name(&self) -> String {
        // N.B. MUST start with "lib" to be recognized by Xcode as a library
        format!("lib{}-rs.xcframework", self.package_name)
    }
}

/// Path builder helper for consistent path construction
struct PathBuilder<'a> {
    crate_path: &'a Path,
}

impl<'a> PathBuilder<'a> {
    fn new(crate_path: &'a Path) -> Self {
        Self { crate_path }
    }

    /// Get path to Cargo.toml
    fn cargo_toml(&self) -> PathBuf {
        self.crate_path.join(paths::CARGO_TOML)
    }

    /// Get rust build directory path
    fn rust_build_dir(&self) -> PathBuf {
        self.crate_path.join(paths::RUST_BUILD_DIR)
    }

    /// Get staging directory path
    fn staging(&self) -> PathBuf {
        self.rust_build_dir()
            .join(paths::XCFRAMEWORK_STAGING_SUBDIR)
    }

    /// Get Swift output directory path (under Rust build dir)
    fn swift_output_dir(&self) -> PathBuf {
        self.rust_build_dir().join(paths::SWIFT_SUBDIR)
    }

    /// Get target library path for given architecture
    fn target_lib(&self, target: &str, package: &str, extension: &str) -> PathBuf {
        self.rust_build_dir().join(format!(
            "{}/{}/lib{}.{}", // N.B. that `release` is used even if not in release mode.
            target,
            paths::RELEASE_SUBDIR,
            package,
            extension
        ))
    }

    /// Get dylib path for given target and package
    fn dylib(&self, target: &str, package: &str) -> PathBuf {
        self.target_lib(target, package, extensions::DYNAMIC_LIB)
    }

    /// Get static library path for given target and package
    fn static_lib(&self, target: &str, package: &str) -> PathBuf {
        self.target_lib(target, package, extensions::STATIC_LIB)
    }

    /// Get apple sources directory using provided path from settings
    fn apple_sources(
        &self,
        apple_sources_dir: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let example_dir = self
            .crate_path
            .parent()
            .ok_or("Cannot find parent directory of crate")?;
        Ok(example_dir.join(apple_sources_dir))
    }

    /// Get Package.swift path (in parent directory)
    fn package_swift(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let example_dir = self
            .crate_path
            .parent()
            .ok_or("Cannot find parent directory of crate")?;
        Ok(example_dir.join(paths::PACKAGE_SWIFT))
    }
}

// ==================== PUBLIC API ====================
pub struct BuildOutcome {
    pub swift_file_path: PathBuf,
    pub path_to_crate: PathBuf,
}

/// Main entry point for building Apple platform bindings
///
/// This function orchestrates the complete build process:
/// 1. Builds Rust libraries for target platforms
/// 2. Generates FFI bindings using UniFFI
/// 3. Creates XCFramework for distribution
/// 4. Post-processes generated Swift code
pub fn build_swift(
    settings: SwiftBuildSettings,
) -> Result<BuildOutcome, Box<dyn std::error::Error>> {
    // Get package information from environment
    let package_name =
        std::env::var(env_vars::CARGO_PKG_NAME).expect("CARGO_PKG_NAME env var should be set");
    let path_to_crate = std::env::var(env_vars::CARGO_MANIFEST_DIR)
        .expect("CARGO_MANIFEST_DIR env var should be set")
        .into();

    let config = BuildConfig {
        package_name,
        path_to_crate,
        settings,
    };

    println!(
        "{} lera_build::build - config {:?}",
        messages::BUILD_START,
        config
    );

    let swift_file_path = build_with_config(&config).expect("Failed to build with default config");

    println!(
        "{} {} build_with_config finished, swift file at: {:?}",
        messages::PACKAGE_BUILD,
        messages::SUCCESS,
        swift_file_path
    );

    Ok(BuildOutcome {
        swift_file_path,
        path_to_crate: config.path_to_crate,
    })
}

// ==================== INTERNAL IMPLEMENTATION ====================

/// Core build orchestration function
///
/// Executes the three main phases of the build process in sequence
fn build_with_config(config: &BuildConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Step 1: Build Rust libraries for all required targets
    RustTargetBuilder::new(config).build_all_targets()?;

    // Step 2: Generate FFI bindings using UniFFI
    let swift_file_path = FFIBindingGenerator::new(config).generate()?;

    // Step 3: Build XCFramework for distribution
    let output = XCFrameworkBuilder::new(config).build()?;

    println!(
        "{} {} End of lera_build::build, output: {}",
        messages::PACKAGE_BUILD,
        messages::SUCCESS,
        output.unwrap_or_else(|| "No release build".to_string())
    );
    Ok(swift_file_path)
}

/// Rust target builder - handles compilation for multiple Apple platforms
struct RustTargetBuilder<'a> {
    config: &'a BuildConfig,
    paths: PathBuilder<'a>,
}

impl<'a> RustTargetBuilder<'a> {
    fn new(config: &'a BuildConfig) -> Self {
        Self {
            config,
            paths: PathBuilder::new(&config.path_to_crate),
        }
    }

    /// Build all required targets based on configuration
    fn build_all_targets(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Building Rust targets for package: {}",
            messages::PACKAGE_BUILD,
            self.config.package_name
        );

        // Always build for macOS
        self.build_target(targets::MACOS)?;

        if !self.config.settings.maconly {
            println!("{} Building iOS and macOS targets", messages::PACKAGE_BUILD);
            self.build_target(targets::IOS_SIM)?;
            self.build_target(targets::IOS)?;
        } else {
            println!(
                "{} Build for macOS only (skipping iOS)",
                messages::PACKAGE_BUILD
            );
        }

        Ok(())
    }

    /// Build a single target architecture
    fn build_target(&self, target: &str) -> Result<(), Box<dyn std::error::Error>> {
        CargoBuilder::new()
            .build_package(&self.config.package_name, &self.paths.cargo_toml(), target)
            .execute()
            .map_err(|e| format!("Failed to build target {}: {}", target, e))?;

        println!(
            "{} Built {} for {}",
            messages::SUCCESS,
            self.config.package_name,
            target
        );
        Ok(())
    }
}

/// FFI binding generator - handles UniFFI Swift binding generation
struct FFIBindingGenerator<'a> {
    config: &'a BuildConfig,
    paths: PathBuilder<'a>,
}

impl<'a> FFIBindingGenerator<'a> {
    fn new(config: &'a BuildConfig) -> Self {
        Self {
            config,
            paths: PathBuilder::new(&config.path_to_crate),
        }
    }

    /// Generate Swift FFI bindings and organize output files
    fn generate(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        println!(
            "{} Generating framework module mapping and FFI bindings for {}",
            messages::FFI_GEN,
            self.config.package_name
        );

        let dylib_path = self
            .paths
            .dylib(self.config.dylib_target(), &self.config.package_name);
        let out_dir = self.paths.staging();

        // Generate Swift bindings using UniFFI
        self.generate_uniffi_bindings(&dylib_path, &out_dir)?;

        // Move generated files to final location
        let swift_file_path = self.organize_generated_files()?;

        println!("{} generate_ffi_bindings finished", messages::SUCCESS);
        Ok(swift_file_path)
    }

    /// Call UniFFI to generate Swift bindings
    fn generate_uniffi_bindings(
        &self,
        dylib_path: &Path,
        out_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        uniffi_bindgen::bindings::generate_swift_bindings(
            uniffi_bindgen::bindings::SwiftBindingsOptions {
                generate_swift_sources: true,
                generate_headers: true,
                generate_modulemap: true,
                source: dylib_path.to_string_lossy().to_string().into(),
                out_dir: out_dir.to_string_lossy().to_string().into(),
                xcframework: false,
                module_name: Some(self.config.module_name()),
                modulemap_filename: Some(paths::MODULE_MAP.to_string()),
                metadata_no_deps: false,
                link_frameworks: Vec::new(),
            },
        )
        .map_err(|e| format!("UniFFI binding generation failed: {}", e))?;

        Ok(())
    }

    /// Move generated Swift files to the Apple project directory
    fn organize_generated_files(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let apple_sources_dir = self
            .paths
            .apple_sources(&self.config.settings.apple_sources_dir)?;
        let staging_dir = self.paths.staging();

        // Create target directory
        fs::create_dir_all(&apple_sources_dir)?;

        let mut swift_file_path: Option<PathBuf> = None;

        // Move Swift files from staging to Apple project
        for entry in fs::read_dir(staging_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some(extensions::SWIFT) {
                let file_name = path.file_name().unwrap();

                if swift_file_path.is_some() {
                    return Err(format!(
                        "Multiple Swift files found in staging directory, not yet supported: {:?}",
                        file_name
                    )
                    .into());
                }

                let target_path = apple_sources_dir.join(file_name);
                swift_file_path = Some(target_path.clone());

                println!(
                    "{} Moving Swift file from {:?} to {:?}",
                    messages::FFI_GEN,
                    path,
                    target_path
                );
                fs::rename(&path, target_path)?;
            }
        }

        swift_file_path.ok_or_else(|| "No Swift file found in staging directory".into())
    }
}

/// XCFramework builder - creates distribution-ready XCFramework
struct XCFrameworkBuilder<'a> {
    config: &'a BuildConfig,
    paths: PathBuilder<'a>,
}

impl<'a> XCFrameworkBuilder<'a> {
    fn new(config: &'a BuildConfig) -> Self {
        Self {
            config,
            paths: PathBuilder::new(&config.path_to_crate),
        }
    }

    /// Build XCFramework with all required architectures
    fn build(&self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        println!(
            "{} Generating XCFramework {}",
            messages::PACKAGE_BUILD,
            self.config.package_name
        );

        let swift_output_dir = self.paths.swift_output_dir();
        let xcframe_path = swift_output_dir.join(self.config.xcframework_name());
        let staging_headers = self.paths.staging();

        // Clean up previous build
        let _ = fs::remove_dir_all(&swift_output_dir);

        // Build XCFramework command
        let mut xcodebuild = Command::new(commands::XCODEBUILD);
        xcodebuild.arg(xcode_args::CREATE_XCFRAMEWORK);

        // Add macOS library (always included)
        let macos_lib = self
            .paths
            .static_lib(targets::MACOS, &self.config.package_name);
        xcodebuild
            .arg(xcode_args::LIBRARY)
            .arg(&macos_lib)
            .arg(xcode_args::HEADERS)
            .arg(&staging_headers);

        // Add iOS libraries if not macOS-only
        if !self.config.settings.maconly {
            let ios_lib = self
                .paths
                .static_lib(targets::IOS, &self.config.package_name);
            let ios_sim_lib = self
                .paths
                .static_lib(targets::IOS_SIM, &self.config.package_name);

            xcodebuild
                .arg(xcode_args::LIBRARY)
                .arg(&ios_lib)
                .arg(xcode_args::HEADERS)
                .arg(&staging_headers)
                .arg(xcode_args::LIBRARY)
                .arg(&ios_sim_lib)
                .arg(xcode_args::HEADERS)
                .arg(&staging_headers);
        }

        xcodebuild.arg(xcode_args::OUTPUT).arg(&xcframe_path);

        // Execute xcodebuild
        let output = xcodebuild.output()?;
        if !output.status.success() {
            return Err(format!(
                "xcodebuild failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Handle release build if needed
        if self.config.is_release() {
            let xcframe_zip_path = format!("{}.zip", xcframe_path.to_string_lossy());
            self.handle_release_build(&xcframe_path.to_string_lossy(), &xcframe_zip_path)
        } else {
            Ok(None)
        }
    }

    /// Handle release build: create ZIP, compute checksum, update Package.swift
    fn handle_release_build(
        &self,
        xcframe_path: &str,
        xcframe_zip_path: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        println!(
            "{} Building xcframework archive for release",
            messages::PACKAGE_BUILD
        );

        // Create ZIP archive
        let output = Command::new(commands::ZIP)
            .args([zip_args::RECURSIVE, xcframe_zip_path, xcframe_path])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "ZIP creation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Compute checksum using Swift Package Manager
        let output = Command::new(commands::SWIFT)
            .args([
                swift_args::PACKAGE,
                swift_args::COMPUTE_CHECKSUM,
                xcframe_zip_path,
            ])
            .output()?;

        if !output.status.success() {
            return Err("Checksum computation failed".into());
        }

        let checksum = String::from_utf8(output.stdout)?.trim().to_string();

        // Update Package.swift with release information
        if let Some(tag) = &self.config.settings.release_tag {
            PackageSwiftUpdater::new(&self.paths).update(tag, &checksum)?;
        }

        Ok(Some(format!("{};{}", checksum, xcframe_zip_path)))
    }
}

/// Package.swift updater - handles release tag and checksum updates
struct PackageSwiftUpdater<'a> {
    paths: &'a PathBuilder<'a>,
}

impl<'a> PackageSwiftUpdater<'a> {
    fn new(paths: &'a PathBuilder<'a>) -> Self {
        Self { paths }
    }

    /// Update Package.swift with new release tag and checksum
    fn update(&self, tag: &str, checksum: &str) -> Result<(), Box<dyn std::error::Error>> {
        let package_swift_path = self.paths.package_swift()?;

        // Read current Package.swift content
        let content = fs::read_to_string(&package_swift_path)?;

        // Update release tag using regex
        let tag_regex = regex::Regex::new(r#"(let releaseTag = ")[^"]+(")"#)?;
        let content = tag_regex.replace(&content, format!("$1{}$2", tag));

        // Update checksum using regex
        let checksum_regex = regex::Regex::new(r#"(let releaseChecksum = ")[^"]+(")"#)?;
        let content = checksum_regex.replace(&content, format!("$1{}$2", checksum));

        // Write updated content back to file
        fs::write(&package_swift_path, content.as_ref())?;

        println!(
            "{} Updated Package.swift with tag: {}, checksum: {}",
            messages::SUCCESS,
            tag,
            checksum
        );

        Ok(())
    }
}
