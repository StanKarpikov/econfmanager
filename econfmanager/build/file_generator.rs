use std::{collections::HashSet, fs::File};
use std::io::Write;
use std::path::Path;

use crate::schema::{self, Group, ParameterValueType};
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

fn format_anyvalue_type(v: &ParameterValueType) -> String {
    match v {
        ParameterValueType::TypeBool => format!("ParameterValueType::TypeBool"),
        ParameterValueType::TypeI32 => format!("ParameterValueType::TypeI32"),
        ParameterValueType::TypeString => format!("ParameterValueType::TypeString"),
        ParameterValueType::TypeU32 => format!("ParameterValueType::TypeU32"),
        ParameterValueType::TypeI64 => format!("ParameterValueType::TypeI64"),
        ParameterValueType::TypeU64 => format!("ParameterValueType::TypeU64"),
        ParameterValueType::TypeF32 => format!("ParameterValueType::TypeF32"),
        ParameterValueType::TypeF64 => format!("ParameterValueType::TypeF64"),
        ParameterValueType::TypeBlob => format!("ParameterValueType::TypeBlob"),
        ParameterValueType::TypeEnum(v) => format!("ParameterValueType::TypeEnum(Cow::Borrowed(\"{}\"))", v),
        ParameterValueType::TypeNone => format!("ParameterValueType::TypeNone"),
    }
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
        ParameterValue::ValEnum(v) => format!("ParameterValue::ValEnum({})", v),
        ParameterValue::ValNone => format!("ParameterValue::ValNone"),
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
        "use crate::schema::{{Parameter, ParameterValue, ParameterValueType, ValidationMethod, Group}};"
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
        let value_type = format_anyvalue_type(&p.value_type);
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

fn value_to_string(value: &ParameterValue) -> String {
    match value {
        ParameterValue::ValBool(b) => b.to_string(),
        ParameterValue::ValI32(i) => i.to_string(),
        ParameterValue::ValU32(u) => u.to_string(),
        ParameterValue::ValI64(i) => i.to_string(),
        ParameterValue::ValU64(u) => u.to_string(),
        ParameterValue::ValF32(f) => f.to_string(),
        ParameterValue::ValF64(f) => f.to_string(),
        ParameterValue::ValEnum(i) => i.to_string(),
        ParameterValue::ValString(s) => s.to_string(),
        ParameterValue::ValBlob(_) => todo!(),
        ParameterValue::ValPath(_) => todo!(),
        ParameterValue::ValNone => "null".to_owned(),
    }
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
    
    let mut enums = HashSet::new();
    
    for p in parameters {
        let pm_enum_name = get_parameter_name_for_enum(&p.name_id.to_string());
        let pm_name = get_parameter_name_for_function(&p.name_id.to_string());
        let pm_type = match &p.value_type {
            ParameterValueType::TypeBool => "bool",
            ParameterValueType::TypeI32 => "i32",
            ParameterValueType::TypeString => "c_char",
            ParameterValueType::TypeU32 => "u32",
            ParameterValueType::TypeI64 => "i64",
            ParameterValueType::TypeU64 => "u64",
            ParameterValueType::TypeF32 => "f32",
            ParameterValueType::TypeF64 => "f64",
            ParameterValueType::TypeBlob => "c_char",
            ParameterValueType::TypeEnum(_) => "i32",
            ParameterValueType::TypeNone => "none",
        };

        writeln!(f, "#[allow(non_camel_case_types)]")?;

        let mut is_enum = false;
        let mut p_enum_name = "";
        if let ParameterValueType::TypeEnum(enum_name) = &p.value_type {
            match &p.validation {
                ValidationMethod::AllowedValues { values, names } => {
                    let vals = values
                        .iter()
                        .map(|v| v)
                        .collect::<Vec<_>>();
                    let str_names = names
                        .iter()
                        .map(|v| v)
                        .collect::<Vec<_>>();

                    if !enums.contains(&enum_name)
                    {
                        enums.insert(enum_name);
                        writeln!(f, "#[repr(i32)]")?;
                        writeln!(f, "pub enum {}_t {{", enum_name)?;
                        for (val, name) in vals.iter().zip(str_names.iter()) {
                            writeln!(f, "    {} = {},", name, value_to_string(val))?;
                        }
                        writeln!(f, "}}\n")?;
                    }
                    p_enum_name = enum_name;
                    is_enum = true;
                }
                _ => todo!("Probably something wrong"),
            };

           
        }else{
            writeln!(f, "pub type {}_t = {}; \n", pm_name, pm_type)?;
        }

        writeln!(f, "#[unsafe(no_mangle)]")?;
        writeln!(f, "pub extern \"C\" fn get_{}(interface: *const CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, if is_enum {p_enum_name} else {&pm_name})?;
        if is_enum {
            writeln!(f, "    let {} = {} as *mut i32;", pm_name, pm_name)?;
        }
        writeln!(f, "    get_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
        writeln!(f, "}}\n")?;

        if !p.is_const {
            writeln!(f, "#[unsafe(no_mangle)]")?;
            writeln!(f, "pub extern \"C\" fn set_{}(interface: *const CInterfaceInstance, {}: *mut {}_t) -> EconfStatus {{", pm_name, pm_name, if is_enum {p_enum_name} else {&pm_name})?;
            if is_enum {
                writeln!(f, "    let {} = {} as *mut i32;", pm_name, pm_name)?;
            }
            writeln!(f, "    set_parameter::<{}>(interface, ParameterId::{}, {})", pm_type, pm_enum_name, pm_name)?;
            writeln!(f, "}}\n")?;
        }
    }

    Ok(())
}
