use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/");

    let udl_path = Path::new("src/counter.udl");
    if udl_path.exists() {
        println!("cargo:rerun-if-changed={}", udl_path.display());
        uniffi::generate_scaffolding(udl_path.to_str().unwrap()).unwrap();
    } else {
        println!(
            "cargo:warning=UDL file {} not found, skipping scaffolding generation",
            udl_path.display()
        );
    }
}
