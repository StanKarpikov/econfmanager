use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::schema::{self, Group};
use schema::{Parameter, ParameterValue, ValidationMethod};


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
        ParameterValue::ValString(s) => format!("ParameterValue::ValString(Cow::Borrowed(\"{}\"))", s),
        ParameterValue::ValU32(u) => format!("ParameterValue::ValU32({})", u),
        ParameterValue::ValI64(i) => format!("ParameterValue::ValI64({})", i),
        ParameterValue::ValU64(u) => format!("ParameterValue::ValU64({})", u),
        ParameterValue::ValF32(f) => format!("ParameterValue::ValF32({}f32)", f),
        ParameterValue::ValF64(f) => format!("ParameterValue::ValF64({}f64)", f),
        ParameterValue::ValBlob(data) => 
                {
                    let bytes_str = data
                        .iter()
                        .map(|b| format!("0x{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("ParameterValue::ValBlob(vec![{}])", bytes_str)
                },
        ParameterValue::ValPath(s) => format!("ParameterValue::ValPath(\"{}\")", s),
    }
}

pub(crate) fn generate_parameter_ids(
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

pub(crate) fn generate_parameter_enum(
    parameters: &Vec<Parameter>,
    groups: &Vec<Group>,
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

    let dest_path = Path::new(&build_dir).join("generated.rs");
    let mut f = File::create(dest_path)?;

    writeln!(f, "use num_enum::TryFromPrimitive;")?;
    writeln!(f, "use std::borrow::Cow;")?;
    writeln!(
        f,
        "use crate::schema::{{Parameter, ParameterValue, ValidationMethod, Group}};"
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
    for p in parameters{
        let value_type = format_anyvalue(&p.value_type);
        let value_default = format_anyvalue(&p.value_default);
        let validation_code = match &p.validation {
            ValidationMethod::None => "ValidationMethod::None".to_string(),
            ValidationMethod::Range { min, max } => format!(
                "ValidationMethod::Range {{ min: {}, max: {} }}",
                format_anyvalue(&min),
                format_anyvalue(&max),
            ),
            ValidationMethod::AllowedValues { values, names } => {
                let vals = values
                    .iter()
                    .map(|v| format_anyvalue(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                let str_names = names
                    .iter()
                    .map(|v| "\"".to_string() + v + "\"")
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "ValidationMethod::AllowedValues {{ values: Cow::Borrowed(&[{}]), names: Cow::Borrowed(&[{}]) }}",
                    vals,
                    str_names
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
        writeln!(f, "            value_type: {},", value_type)?;
        writeln!(f, "            value_default: {},", value_default)?;
        // writeln!(f, "            id: {:?},", enum_variants[idx])?;
        writeln!(f, "            name_id: {:?},", p.name_id)?;
        writeln!(f, "            validation: {},", validation_code)?;
        writeln!(f, "            comment: {:?},", p.comment)?;
        writeln!(f, "            title: {:?},", p.title)?;
        writeln!(f, "            is_const: {},", p.is_const)?;
        writeln!(f, "            tags: vec![{}],", tags_code)?;
        writeln!(f, "            runtime: {},", p.runtime)?;
        writeln!(f, "        }},")?;
    }
    writeln!(f, "];\n\n")?;

    writeln!(f, "pub const GROUPS_DATA: &'static [Group] = &[")?;
    for g in groups{
        writeln!(f, "        Group {{")?;
        writeln!(f, "            name: {:?},", g.name)?;
        writeln!(f, "            title: {:?},", g.title)?;
        writeln!(f, "            comment: {:?},", g.comment)?;
        writeln!(f, "        }},")?;
    }

    writeln!(f, "];")?;

    Ok(())
}

pub(crate) fn generate_parameter_functions(
    parameters: &Vec<Parameter>,
    build_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let dest_path = Path::new(&build_dir).join("parameter_functions.rs");
    let mut f = File::create(dest_path)?;

    writeln!(f, "/// Auto‐generated. See build.rs\n")?;
    
    writeln!(f, "use std::ffi::c_char;")?;
    writeln!(f, "use crate::{{lib_helper_functions::{{get_parameter, set_parameter}}, generated::ParameterId, CInterfaceInstance, EconfStatus}};\n")?;
    
    for p in parameters {
        let pm_enum_name = get_parameter_name_for_enum(&p.name_id.to_string());
        let pm_name = get_parameter_name_for_function(&p.name_id.to_string());
        let pm_type = match &p.value_type {
            ParameterValue::ValBool(_) => "bool",
            ParameterValue::ValI32(_) => "i32",
            ParameterValue::ValString(_) => "c_char",
            ParameterValue::ValU32(_) => "u32",
            ParameterValue::ValI64(_) => "i64",
            ParameterValue::ValU64(_) => "u64",
            ParameterValue::ValF32(_) => "f32",
            ParameterValue::ValF64(_) => "f64",
            ParameterValue::ValBlob(_) => "c_char",
            ParameterValue::ValPath(_) => "c_char",
        };

        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "pub type {}_t = {}; \n", pm_name, pm_type)?;

        writeln!(f, "#[unsafe(no_mangle)]")?;
        writeln!(f, "pub extern \"C\" fn get_{}(interface: *const CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, pm_name)?;
        writeln!(f, "    get_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
        writeln!(f, "}}\n")?;

        writeln!(f, "#[unsafe(no_mangle)]")?;
        writeln!(f, "pub extern \"C\" fn set_{}(interface: *const CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, pm_name)?;
        writeln!(f, "    set_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
        writeln!(f, "}}\n")?;
    }

    Ok(())
}
