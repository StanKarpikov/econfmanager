use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let descriptor_path = out_dir.join("descriptors.bin");
    
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