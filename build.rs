use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "src/schema.rs"] pub mod schema;
use schema::{Parameter, SchemaManager};
// #[path = "src/configfile.rs"] pub mod config;
// use config::Config;

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
    
    println!("cargo:rerun-if-changed=configuration.proto");
    println!("cargo:rerun-if-changed=configuration_options.proto");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir
        .ancestors()
        .nth(3) // OUT_DIR is like target/debug/build/crate-hash/out
        .expect("Failed to find build directory");

    // let config = Config::new(descriptor_path.to_str().unwrap().to_owned(),
    //                                  protofile_path.to_str().unwrap().to_owned(), 
    //                                  "".to_owned())?;
    let schema = SchemaManager::new(descriptor_path.to_str().unwrap().to_owned(), "configuration.proto".to_owned())?;
    generate_parameter_enum(schema.get_parameters()?, build_dir.to_str().unwrap().to_owned());

    let header_path = build_dir.join("econfmanager.h");
    let status = Command::new("cbindgen")
        .arg("--crate")
        .arg("econfmanager") 
        .arg("--output")
        .arg(header_path)
        .status()
        .expect("Failed to run cbindgen");

    if !status.success() {
        panic!("cbindgen failed with status: {}", status);
    }

    Ok(())
}

fn get_parameter_name_for_enum(name_id: &String) -> String {
    // name_id
    //     .chars()
    //     .map(|c| {
    //         // if c == '_' {
    //         //     '_'
    //         // } else if c == '@' {
    //         //     '_'
    //         // } else {
    //             c
    //         // }
    //     })
    //     .collect()
    name_id.split('@')
            .map(|part| part.to_uppercase())
            .collect::<Vec<_>>()
            .join("_")
}

fn generate_parameter_enum(parameters: Vec<Parameter>, build_dir: String) {

    let enum_variants: Vec<String> = parameters.iter().map(|parameter| format!("    {},", get_parameter_name_for_enum(&parameter.name_id))).collect();
    let array_entries: Vec<String> = parameters.iter().map(|parameter| format!("    \"{}\",", parameter.name_id)).collect();

    let output = format!(
        r#"
#[repr(C)]
pub enum Parameters {{
{enum_variants}
}}

#[repr(C)]
pub const PARAMETER_ID: &[&str] = &[
{array_entries}
];
"#,
        enum_variants = enum_variants.join("\n"),
        array_entries = array_entries.join("\n"),
    );

    let dest_path = Path::new(&build_dir).join("generated.rs");
    fs::write(dest_path, output).unwrap();
}