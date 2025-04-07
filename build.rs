use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_path = Path::new(&project_root).join("proto");
    let descriptor_path = proto_path.join("descriptors.bin");
    fs::create_dir_all(proto_path)?;

    // Run protoc to generate the descriptor set
    let status = Command::new("protoc")
        .arg("--include_imports")
        .arg("--descriptor_set_out")
        .arg(&descriptor_path)
        .arg("--proto_path=proto_conf")
        .arg("configuration.proto")
        .arg("configuration_options.proto")
        .status()?;
    
    if !status.success() {
        return Err("protoc failed to generate descriptors".into());
    }
    
    // Tell Cargo to rerun if proto files change
    println!("cargo:rerun-if-changed=configuration.proto");
    println!("cargo:rerun-if-changed=configuration_options.proto");
    
    Ok(())
}