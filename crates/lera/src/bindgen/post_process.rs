use super::{post_process_kotlin::kotlin_transform, post_process_swift::swift_transform};

use std::{fs, path::Path};

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("Failed to read path: '{:?}', error: {:?}", path, e))
}

fn write(path: &Path, contents: String) -> Result<(), String> {
    let size = contents.len();
    fs::write(path, &contents)
        .map_err(|e| format!("Failed to write to path '{:?}', error: {:?}", path, e))
        .inspect(|_| {
            println!(
                "🔮 Replaced: '{:?}' with post processed contents (#{} bytes). ✨",
                path, size
            )
        })
}

fn process_file(
    generated_path: &Path,
    crate_path: &Path,
    expected_extension: &str,
    label: &str,
    transform: impl Fn(String, &Path) -> Result<String, String>,
) -> Result<(), String> {
    assert!(
        generated_path.exists(),
        "Generated file {:?} must exist",
        generated_path
    );
    assert!(
        crate_path.exists() && crate_path.is_dir(),
        "crate path {:?} must exist",
        crate_path
    );

    let extension_ok = generated_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{ext}") == expected_extension)
        .unwrap_or(false);

    assert!(
        extension_ok,
        "Expected {:?} to end with {}",
        generated_path, expected_extension
    );

    println!(
        "🔮 starting post processing: {} file: {:?}, rust crate: {:?}",
        label, generated_path, crate_path
    );

    let contents = read(generated_path)?;
    let transformed = transform(contents, crate_path)?;
    write(generated_path, transformed)?;

    println!("🔮 post processing done for {}. ✔", label);
    Ok(())
}

pub fn post_process_swift(generated_path: &Path, crate_path: &Path) {
    process_file(
        generated_path,
        crate_path,
        ".swift",
        "swift",
        swift_transform,
    )
    .unwrap();
}

pub fn post_process_kotlin(generated_path: &Path, crate_path: &Path) {
    process_file(
        generated_path,
        crate_path,
        ".kt",
        "kotlin",
        kotlin_transform,
    )
    .unwrap();
}

/// Backwards compatibility helper for existing Swift build callers.
#[allow(dead_code)]
pub fn post_process(generated_path: &Path, crate_path: &Path) {
    post_process_swift(generated_path, crate_path);
}
