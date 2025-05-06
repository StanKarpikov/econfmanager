use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use std::fs::canonicalize;

#[path = "build/file_generator.rs"]
pub mod file_generator;

#[path = "src/schema.rs"]
pub mod schema;
use file_generator::{generate_parameter_enum, generate_parameter_functions, generate_parameter_ids};
use schema::SchemaManager;

const OPTIONS_PROTO_FILE: &str = "options.proto";
const PARAMETERS_PROTO_FILE: &str = "parameters.proto";
const SERVICE_PROTO_FILE: &str = "services.proto";
const SERVICE_PROTO_FILE_RS: &str = "services.rs";
const PARAMETER_IDS_FILE: &str = "parameter_ids.proto";
const PARAMETER_IDS_PROTO_FILE_RS: &str = "parameter_ids.rs";
const DESCRIPTORS_FILE: &str = "descriptors.bin";

const PROTO_CONF_FOLDER: &str = "proto_conf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parameters_proto_path = env::var("PARAMETERS_PROTO_PATH").unwrap_or_else(|_| {
        eprintln!("Environment parameter PARAMETERS_PROTO_PATH not set, using default EXAMPLES path");
        "../examples/peripheral_service/proto".to_owned()
    });
    
    if !Path::new(&parameters_proto_path).exists() {
        panic!("Parameters proto folder not found at: {}", parameters_proto_path);
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("no variable called OUT_DIR"));
    // let project_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let generated_proto_path = Path::new(&out_dir);
    let parameters_proto_flepath = Path::new(&parameters_proto_path).join(PARAMETERS_PROTO_FILE);

    println!("cargo:rustc-env=SERVICE_PROTO_FILE_RS={SERVICE_PROTO_FILE_RS}");
    println!("cargo:rustc-env=CONFIGURATION_PROTO_FILE={PARAMETERS_PROTO_FILE}");
    println!("cargo:rustc-env=PARAMETER_IDS_PROTO_FILE_RS={PARAMETER_IDS_PROTO_FILE_RS}");

    fs::create_dir_all(generated_proto_path)
        .unwrap_or_else(|op|{panic!("Failed creating output dirs: {}", op)});

    let abs_descriptor_path = canonicalize(generated_proto_path)
        .unwrap_or_else(|op|{panic!("Error getting path for generated files folder: {}", op)})
        .join(DESCRIPTORS_FILE);
    let abs_parameters_path = canonicalize(&parameters_proto_path)
        .unwrap_or_else(|op|{panic!("Error getting path for parameters file: {}", op)});
    let abs_proto_conf_path = canonicalize(PROTO_CONF_FOLDER)
        .unwrap_or_else(|op|{panic!("Error getting path for proto_conf file: {}", op)});

    // Run protoc to generate the descriptor set
    let mut cmd = Command::new("protoc");
    cmd.arg("--include_imports")
        .arg("--descriptor_set_out")
        .arg(&abs_descriptor_path)
        .arg(format!("--proto_path={}", abs_parameters_path.display()))
        .arg(format!("--proto_path={}", abs_proto_conf_path.display()))
        .arg(PARAMETERS_PROTO_FILE)
        .arg(OPTIONS_PROTO_FILE);
    eprintln!("Executing protoc: {:?}", cmd);
    let status = cmd.status()
        .unwrap_or_else(|op|{panic!("Error getting protoc command exit status: {}", op)});
    if !status.success() {
        panic!("protoc failed to generate descriptors");
    }

    println!("cargo:rerun-if-changed={}", parameters_proto_flepath.to_string_lossy());
    println!("cargo:rerun-if-changed={}", OPTIONS_PROTO_FILE);
    println!("cargo:rerun-if-changed={}", SERVICE_PROTO_FILE);

    let build_dir = out_dir
        .ancestors()
        .nth(3) // OUT_DIR is like target/debug/build/crate-hash/out
        .expect("Failed to find build directory");

    let generated_dir_path = build_dir.ancestors().nth(1).expect("Failed to find generated files directory").join("generated");
    let generated_dir = generated_dir_path.as_path();
    // println!("cargo:rustc-env=GENERATED_FILES_DIR={}", generated_dir.to_str().unwrap().to_owned());
    fs::create_dir_all(generated_dir)
        .unwrap_or_else(|op|{panic!("Failed creating generated files dir: {}", op)});

    let schema = SchemaManager::new(
        abs_descriptor_path.into_os_string().into_string().unwrap(),
        Vec::new(),
        PARAMETERS_PROTO_FILE.to_owned(),
    )
        .unwrap_or_else(|op|{panic!("Error creating schema: {}", op)});

    let (parameters, groups) = schema.get_parameters()
        .unwrap_or_else(|op|{panic!("Error getting parameters list: {}", op)});

    generate_parameter_ids(&parameters, build_dir.to_str().unwrap().to_owned())
        .unwrap_or_else(|op|{panic!("Error generating parameters ids: {}", op)});

    generate_parameter_enum(&parameters, &groups, generated_dir.to_str().unwrap().to_owned())
        .unwrap_or_else(|op|{panic!("Error generating parameters enum: {}", op)});

    generate_parameter_functions(&parameters, generated_dir.to_str().unwrap().to_owned())
        .unwrap_or_else(|op|{panic!("Error generating parameters functions: {}", op)});

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

    prost_build::compile_protos(
        &[
            SERVICE_PROTO_FILE,
            PARAMETERS_PROTO_FILE,
            PARAMETER_IDS_FILE,
        ],
        &[
            build_dir.to_str().unwrap(), 
            abs_parameters_path.to_str().unwrap(), 
            abs_proto_conf_path.to_str().unwrap()
        ],
    )
        .unwrap_or_else(|op|{panic!("Error compiling protos: {}", op)});
    // eprintln!("path = {}", out_dir.to_str().unwrap());
    Ok(())
}
