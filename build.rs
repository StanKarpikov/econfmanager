use std::fs::File;
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;

#[path = "src/schema.rs"] pub mod schema;
use schema::{ParameterValue, Parameter, SchemaManager, ValidationMethod};
// #[path = "src/configfile.rs"] pub mod config;
// use config::Config;

const OPTIONS_PROTO_FILE: &str = "options.proto";
const PARAMETERS_PROTO_FILE: &str = "parameters.proto";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::var("OUT_DIR").expect("no variable called OUT_DIR");
    let out_dir = PathBuf::from(path);
    // let project_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_path = Path::new(&out_dir);
    let descriptor_path = proto_path.join("descriptors.bin");
    println!("cargo:rustc-env=CONFIGURATION_PROTO_FILE={PARAMETERS_PROTO_FILE}");
    fs::create_dir_all(proto_path)?;

    // Run protoc to generate the descriptor set
    let status = Command::new("protoc")
        .arg("--include_imports")
        .arg("--descriptor_set_out")
        .arg(&descriptor_path)
        .arg("--proto_path=proto_conf")
        .arg(PARAMETERS_PROTO_FILE)
        .arg(OPTIONS_PROTO_FILE)
        .status()?;
    
    if !status.success() {
        return Err("protoc failed to generate descriptors".into());
    }
    
    println!("cargo:rerun-if-changed={}", PARAMETERS_PROTO_FILE);
    println!("cargo:rerun-if-changed={}", OPTIONS_PROTO_FILE);

    let build_dir = out_dir
        .ancestors()
        .nth(3) // OUT_DIR is like target/debug/build/crate-hash/out
        .expect("Failed to find build directory");

    // let config = Config::new(descriptor_path.to_str().unwrap().to_owned(),
    //                                  protofile_path.to_str().unwrap().to_owned(), 
    //                                  "".to_owned())?;
    let schema = SchemaManager::new(descriptor_path.to_str().unwrap().to_owned(), Vec::new(), PARAMETERS_PROTO_FILE.to_owned())?;
    let parameters = schema.get_parameters()?;
    generate_parameter_enum(&parameters, build_dir.to_str().unwrap().to_owned())?;

    generate_parameter_ids(&parameters, build_dir.to_str().unwrap().to_owned())?;

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
    name_id.split('@')
            .map(|part| part.to_uppercase())
            .collect::<Vec<_>>()
            .join("_")
}

fn format_anyvalue(v: &ParameterValue) -> String {
    match v {
        ParameterValue::ValBool(b)   => format!("ParameterValue::ValBool({})", b),
        ParameterValue::ValI32(i)    => format!("ParameterValue::ValI32({})", i),
        ParameterValue::ValString(s) => format!("ParameterValue::ValString(String::from({:?}))", s),
        ParameterValue::ValU32(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValI64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValU64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValF32(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValF64(_) => format!("ParameterValue::ValI32(0)"),
        ParameterValue::ValBlob(items) => format!("ParameterValue::ValI32(0)"),
    }
}

fn generate_parameter_ids(parameters: &Vec<Parameter>, build_dir: String)  -> Result<(), Box<dyn std::error::Error>> {
    let enum_variants: Vec<String> = parameters.iter().map(|parameter| format!("    {}", get_parameter_name_for_enum(&parameter.name_id.to_string()))).collect();

    let dest_path = Path::new(&build_dir).join("parameter_ids.proto");
    let mut f = File::create(dest_path)?;

    writeln!(f, "// Auto-generated. See build.rs")?;
    writeln!(f, "syntax = \"proto3\";")?;
    writeln!(f, "")?;
    writeln!(f, "enum ParameterId {{")?;
    for (index, variant) in enum_variants.iter().enumerate() {
        writeln!(f, "{} = {};", variant, index)?;
    }
    writeln!(f, "}}")?;
    Ok(())
}

fn generate_parameter_enum(parameters: &Vec<Parameter>, build_dir: String)  -> Result<(), Box<dyn std::error::Error>> {
    let enum_variants: Vec<String> = parameters.iter().map(|parameter| format!("{}", get_parameter_name_for_enum(&parameter.name_id.to_string()))).collect();
    let array_entries: Vec<String> = parameters.iter().map(|parameter| format!("    \"{}\",", parameter.name_id)).collect();

    let dest_path = Path::new(&build_dir).join("generated.rs");
    let mut f = File::create(dest_path)?;
    
    writeln!(f, "use super::*;")?;
    writeln!(f, "use num_enum::TryFromPrimitive;")?;
    writeln!(f, "use crate::schema::{{ParameterValue, ValidationMethod}};")?;
    writeln!(f, "/// Auto‐generated. See build.rs")?;

    writeln!(f, "#[repr(usize)]")?;
    writeln!(f, "#[derive(TryFromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]")?;
    writeln!(f, "#[allow(non_camel_case_types)]")?;
    writeln!(f, "pub enum Parameters {{")?;
    for (index, variant) in enum_variants.iter().enumerate() {
        writeln!(f, "    {} = {},", variant, index)?;
    }
    writeln!(f, "    INVALID_PARAMETER,")?;
    writeln!(f, "}}")?;
    // writeln!(f, "pub const PARAMETER_ID: &[&str] = &[")?;
    // writeln!(f, "{}", array_entries.join("\n"))?;
    // writeln!(f, "];")?;

    writeln!(f, "pub const PARAMETER_DATA: &'static [Parameter] = &[")?;

    for p in parameters {
        // For each field we serialize into Rust syntax:
        let value_code = match &p.value {
            ParameterValue::ValBool(b)   => format!("ParameterValue::ValBool({})", b),
            ParameterValue::ValI32(i)    => format!("ParameterValue::ValI32({})", i),
            ParameterValue::ValString(s) => format!("ParameterValue::ValString(String::from({:?}))", s),
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
                        let vals = values.iter()
                            .map(|v| format_anyvalue(v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("ValidationMethod::AllowedValues {{ values: vec![{}] }}", vals)
                    }
            ValidationMethod::CustomCallback => todo!(),
        };
        let tags_code = p.tags
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "        Parameter {{")?;
        writeln!(f, "            value: {},",     value_code)?;
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