use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use std::fs::canonicalize;

#[path = "src/schema.rs"]
pub mod schema;
use schema::{Parameter, ParameterValue, SchemaManager, ValidationMethod};
// #[path = "src/configfile.rs"] pub mod config;
// use config::Config;

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
        panic!("Environment parameter PARAMETERS_PROTO_PATH not set");
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

    fs::create_dir_all(generated_proto_path)?;

    let abs_descriptor_path = canonicalize(generated_proto_path.join(DESCRIPTORS_FILE))?;
    let abs_parameters_path = canonicalize(&parameters_proto_path)?;
    let abs_proto_conf_path = canonicalize(PROTO_CONF_FOLDER)?;

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
    let status = cmd.status()?;
    if !status.success() {
        return Err("protoc failed to generate descriptors".into());
    }

    println!("cargo:rerun-if-changed={}", parameters_proto_flepath.to_string_lossy());
    println!("cargo:rerun-if-changed={}", OPTIONS_PROTO_FILE);
    println!("cargo:rerun-if-changed={}", SERVICE_PROTO_FILE);

    let build_dir = out_dir
        .ancestors()
        .nth(3) // OUT_DIR is like target/debug/build/crate-hash/out
        .expect("Failed to find build directory");

    // let config = Config::new(descriptor_path.to_str().unwrap().to_owned(),
    //                                  protofile_path.to_str().unwrap().to_owned(),
    //                                  "".to_owned())?;
    let schema = SchemaManager::new(
        abs_descriptor_path.into_os_string().into_string().unwrap(),
        Vec::new(),
        PARAMETERS_PROTO_FILE.to_owned(),
    )?;
    let parameters = schema.get_parameters()?;
    generate_parameter_enum(&parameters, build_dir.to_str().unwrap().to_owned())?;

    generate_parameter_ids(&parameters, build_dir.to_str().unwrap().to_owned())?;

    generate_parameter_functions(&parameters, build_dir.to_str().unwrap().to_owned())?;

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
    )?;
    // eprintln!("path = {}", out_dir.to_str().unwrap());
    Ok(())
}

fn get_parameter_name_for_enum(name_id: &String) -> String {
    name_id
        .split('@')
        .map(|part| part.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

fn get_parameter_name_for_function(name_id: &String) -> String {
    name_id.split('@').collect::<Vec<_>>().join("_")
}

fn format_anyvalue(v: &ParameterValue) -> String {
    match v {
        ParameterValue::ValBool(b) => format!("ParameterValue::ValBool({})", b),
        ParameterValue::ValI32(i) => format!("ParameterValue::ValI32({})", i),
        ParameterValue::ValString(s) => format!("ParameterValue::ValString(String::from({:?}))", s),
        ParameterValue::ValU32(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValI64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValU64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValF32(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValF64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValBlob(_) => format!("ParameterValue::ValI32(0)"),
    }
}

fn generate_parameter_ids(
    parameters: &Vec<Parameter>,
    build_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let enum_variants: Vec<String> = parameters
        .iter()
        .map(|parameter| {
            format!(
                "    {}",
                get_parameter_name_for_enum(&parameter.name_id.to_string())
            )
        })
        .collect();

    let dest_path = Path::new(&build_dir).join("parameter_ids.proto");
    let mut f = File::create(dest_path)?;

    writeln!(f, "// Auto-generated. See build.rs")?;
    writeln!(f, "syntax = \"proto3\";")?;
    writeln!(f, "package parameter_ids;")?;
    writeln!(f, "enum ParameterIdApi {{")?;
    for (index, variant) in enum_variants.iter().enumerate() {
        writeln!(f, "{} = {};", variant, index)?;
    }
    writeln!(f, "}}")?;
    Ok(())
}

fn generate_parameter_enum(
    parameters: &Vec<Parameter>,
    build_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let enum_variants: Vec<String> = parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}",
                get_parameter_name_for_enum(&parameter.name_id.to_string())
            )
        })
        .collect();
    // let array_entries: Vec<String> = parameters
    //     .iter()
    //     .map(|parameter| format!("    \"{}\",", parameter.name_id))
    //     .collect();

    let dest_path = Path::new(&build_dir).join("generated.rs");
    let mut f = File::create(dest_path)?;

    writeln!(f, "use super::*;")?;
    writeln!(f, "use num_enum::TryFromPrimitive;")?;
    writeln!(
        f,
        "use crate::schema::{{ParameterValue, ValidationMethod}};"
    )?;
    writeln!(f, "/// Auto‐generated. See build.rs")?;

    writeln!(f, "#[repr(usize)]")?;
    writeln!(
        f,
        "#[derive(TryFromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]"
    )?;
    writeln!(f, "#[allow(non_camel_case_types)]")?;
    writeln!(f, "pub enum ParameterId {{")?;
    for (index, variant) in enum_variants.iter().enumerate() {
        writeln!(f, "    {} = {},", variant, index)?;
    }
    writeln!(f, "    INVALID_PARAMETER")?;
    writeln!(f, "}}\n")?;

    writeln!(f, "pub const PARAMETERS_NUM:usize = {};\n", enum_variants.len())?;

    writeln!(f, "pub const PARAMETER_DATA: &'static [Parameter] = &[")?;

    for p in parameters {
        let value_code = match &p.value {
            ParameterValue::ValBool(b) => format!("ParameterValue::ValBool({})", b),
            ParameterValue::ValI32(i) => format!("ParameterValue::ValI32({})", i),
            ParameterValue::ValString(s) => {
                format!("ParameterValue::ValString(String::from({:?}))", s)
            }
            ParameterValue::ValU32(_) => format!("ParameterValue::ValI32(0)"),
            ParameterValue::ValI64(_) => format!("ParameterValue::ValI32(0)"),
            ParameterValue::ValU64(_) => format!("ParameterValue::ValI32(0)"),
            ParameterValue::ValF32(_) => format!("ParameterValue::ValI32(0)"),
            ParameterValue::ValF64(_) => format!("ParameterValue::ValI32(0)"),
            ParameterValue::ValBlob(_) => format!("ParameterValue::ValI32(0)"),
        };
        let validation_code = match &p.validation {
            ValidationMethod::None => "ValidationMethod::None".to_string(),
            ValidationMethod::Range { min, max } => format!(
                "ValidationMethod::Range {{ min: {}, max: {} }}",
                format_anyvalue(&min),
                format_anyvalue(&max),
            ),
            ValidationMethod::AllowedValues { values } => {
                let vals = values
                    .iter()
                    .map(|v| format_anyvalue(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "ValidationMethod::AllowedValues {{ values: vec![{}] }}",
                    vals
                )
            }
            ValidationMethod::CustomCallback => todo!(),
        };
        let tags_code = p
            .tags
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "        Parameter {{")?;
        writeln!(f, "            value: {},", value_code)?;
        writeln!(f, "            name_id: {:?},", p.name_id)?;
        writeln!(f, "            validation: {},", validation_code)?;
        writeln!(f, "            comment: {:?},", p.comment)?;
        writeln!(f, "            is_const: {},", p.is_const)?;
        writeln!(f, "            tags: vec![{}],", tags_code)?;
        writeln!(f, "        }},")?;
    }

    writeln!(f, "];")?;

    Ok(())
}

fn generate_parameter_functions(
    parameters: &Vec<Parameter>,
    build_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let dest_path = Path::new(&build_dir).join("parameter_functions.rs");
    let mut f = File::create(dest_path)?;

    writeln!(f, "/// Auto‐generated. See build.rs\n")?;
    
    writeln!(f, "use crate::{{lib_helper_functions::{{get_parameter, set_parameter}}, interface::generated::ParameterId, CInterfaceInstance, EconfStatus}};\n")?;
    
    for p in parameters {
        let pm_enum_name = get_parameter_name_for_enum(&p.name_id.to_string());
        let pm_name = get_parameter_name_for_function(&p.name_id.to_string());
        let pm_type = match &p.value {
            ParameterValue::ValBool(_) => "bool",
            ParameterValue::ValI32(_) => "i32",
            ParameterValue::ValString(_) => "c_char",
            ParameterValue::ValU32(_) => "u32",
            ParameterValue::ValI64(_) => "i64",
            ParameterValue::ValU64(_) => "u64",
            ParameterValue::ValF32(_) => "f32",
            ParameterValue::ValF64(_) => "f64",
            ParameterValue::ValBlob(_) => "c_char",
        };

        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "pub type {}_t = {}; \n", pm_name, pm_type)?;

        writeln!(f, "#[unsafe(no_mangle)]")?;
        writeln!(f, "pub extern \"C\" fn get_{}(interface: CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, pm_name)?;
        writeln!(f, "    get_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
        writeln!(f, "}}\n")?;

        writeln!(f, "#[unsafe(no_mangle)]")?;
        writeln!(f, "pub extern \"C\" fn set_{}(interface: CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, pm_name)?;
        writeln!(f, "    set_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
        writeln!(f, "}}\n")?;
    }

    Ok(())
}
