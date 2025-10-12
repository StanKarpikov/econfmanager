use std::fs;
use std::process::Command;
use std::{collections::HashSet, fs::File};
use std::io::Write;
use std::path::Path;

use crate::schema::{self, Group, ParameterValueType};
use regex::Regex;
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

fn get_parameter_name_short(name_id: &String) -> String {
    name_id.split('@').nth(1).unwrap_or(name_id).to_string()
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
        writeln!(f, "            tags: Cow::Borrowed(&[{}]),", tags_code)?;
        writeln!(f, "            runtime: {},", p.runtime)?;
        writeln!(f, "            readonly: {},", p.readonly)?;
        writeln!(f, "            internal: {},", p.internal)?;
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
    {
        let mut f = File::create(dest_path.clone())?;

        writeln!(f, "/// Auto‐generated. See build.rs\n")?;
        
        writeln!(f, "use std::ffi::c_char;")?;
        writeln!(f, "#[allow(unused_imports)]")?;
        writeln!(f, "use crate::{{")?;
        writeln!(f, "lib_helper_functions::{{get_parameter, get_parameter_quick, set_parameter, get_string, set_string, get_blob, set_blob}}, generated::ParameterId, CInterfaceInstance, EconfStatus}};\n")?;
        writeln!(f, "use num_derive::FromPrimitive;")?;
        writeln!(f, "use num_traits::FromPrimitive;")?;

        let mut enums = HashSet::new();
        
        for p in parameters {
            let pm_name = get_parameter_name_for_function(&p.name_id.to_string());
            let pm_id_name = get_parameter_name_for_enum(&p.name_id.to_string());
            let short_name = get_parameter_name_short(&p.name_id.to_string());

            match &p.value_type {
                ParameterValueType::TypeNone => todo!(),
                ParameterValueType::TypeBool => write_general_setter_and_getter(&mut f, "bool".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeI32 => write_general_setter_and_getter(&mut f, "i32".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeU32 => write_general_setter_and_getter(&mut f, "u32".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeI64 => write_general_setter_and_getter(&mut f, "i64".to_owned(), pm_name,short_name, pm_id_name,  p.is_const)?,
                ParameterValueType::TypeU64 => write_general_setter_and_getter(&mut f, "u64".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeF32 => write_general_setter_and_getter(&mut f, "f32".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeF64 => write_general_setter_and_getter(&mut f, "f64".to_owned(), pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeString => write_string_setter_and_getter(&mut f, pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeBlob => write_blob_setter_and_getter(&mut f, pm_name, short_name, pm_id_name, p.is_const)?,
                ParameterValueType::TypeEnum(p_enum_name) => write_enum_setter_and_getter(&mut f, p_enum_name.to_string(), pm_name, short_name, pm_id_name, p.is_const, &p.validation, &mut enums)?,
            }
        }
    }

    Command::new("rustfmt")
        .arg(dest_path)
        .status()?;

    Ok(())
}

fn write_string_setter_and_getter(f: &mut File, pm_name: String, short_name: String, pm_id_name: String, is_const: bool) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(f, r#"
        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}(
            interface: *const CInterfaceInstance,
            {short_name}: *mut c_char,
            max_len: usize,
            out_len: *mut usize
        ) -> EconfStatus {{
            get_string(interface, ParameterId::{pm_id_name}, {short_name}, max_len, out_len)
        }}
    "#)?;
            
    if !is_const {
        writeln!(f, r#"
            #[unsafe(no_mangle)]
            pub extern "C" fn set_{pm_name}(
                interface: *const CInterfaceInstance,
                {short_name}: *const c_char
            ) -> EconfStatus {{
                set_string(interface, ParameterId::{pm_id_name}, {short_name})
            }}
        "#)?;
    }

    Ok(())
}

fn write_blob_setter_and_getter(f: &mut File, pm_name: String, short_name: String, pm_id_name: String, is_const: bool) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(f, r#"
        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}(
            interface: *const CInterfaceInstance,
            {short_name}: *mut u8,
            max_len: usize,
            out_len: *mut usize,
        ) -> EconfStatus {{
            get_blob(interface, ParameterId::{pm_id_name}, {short_name}, max_len, out_len)
        }}
    "#)?;
            
    if !is_const {
        writeln!(f, r#"
            #[unsafe(no_mangle)]
            pub extern "C" fn set_{pm_name}(
                interface: *const CInterfaceInstance,
                {short_name}: *const u8,
                len: usize
            ) -> EconfStatus {{
                set_blob(interface, ParameterId::{pm_id_name}, {short_name}, len)
            }}
        "#)?;
    }

    Ok(())
}

fn write_general_setter_and_getter(f: &mut File, pm_type: String, pm_name: String, short_name: String, pm_id_name: String, is_const: bool) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(f, r#"
        #[allow(non_camel_case_types)]
        pub type {pm_name}_t = {pm_type};

        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}(
            interface: *const CInterfaceInstance,
            {short_name}: *mut {pm_name}_t
        ) -> EconfStatus {{
            get_parameter::<{pm_type}>(interface, ParameterId::{pm_id_name}, {short_name})
        }}

        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}_quick(
            interface: *const CInterfaceInstance
        ) -> {pm_name}_t {{
            get_parameter_quick::<{pm_type}>(interface, ParameterId::{pm_id_name})
        }}
    "#)?;
            
    if !is_const {
        writeln!(f, r#"
            #[unsafe(no_mangle)]
            pub extern "C" fn set_{pm_name}(
                interface: *const CInterfaceInstance,
                {short_name}: {pm_name}_t,
                {short_name}_result: *mut {pm_name}_t
            ) -> EconfStatus {{
                set_parameter::<{pm_type}>(interface, ParameterId::{pm_id_name}, {short_name}, {short_name}_result)
            }}
        "#)?;
    }

    Ok(())
}

fn write_enum_setter_and_getter(f: &mut File, p_enum_name: String, pm_name: String, short_name: String, pm_id_name: String, is_const: bool, validation: &ValidationMethod, enums: &mut HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
    match &validation {
        ValidationMethod::AllowedValues { values, names } => {
            let vals = values
                .iter()
                .map(|v| v)
                .collect::<Vec<_>>();
            let str_names = names
                .iter()
                .map(|v| v)
                .collect::<Vec<_>>();

            if !enums.contains(&p_enum_name)
            {
                enums.insert(p_enum_name.clone());
                writeln!(f, "#[repr(i32)]")?;
                writeln!(f, "#[allow(non_camel_case_types)]")?;
                writeln!(f, "#[allow(non_local_definitions)]")?;
                writeln!(f, "#[derive(Default, FromPrimitive)]")?;
                writeln!(f, "pub enum {p_enum_name}_t {{")?;
                writeln!(f, "#[default]")?;
                for (val, name) in vals.iter().zip(str_names.iter()) {
                    writeln!(f, "    {} = {},", name, value_to_string(val))?;
                }
                writeln!(f, "}}\n")?;
            }
        }
        _ => todo!("Probably something wrong"),
    };  

    writeln!(f, r#"
        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}(
            interface: *const CInterfaceInstance,
            {short_name}: *mut {p_enum_name}_t
        ) -> EconfStatus {{
            let parameter_i32 = {short_name} as *mut i32;
            get_parameter::<i32>(interface, ParameterId::{pm_id_name}, parameter_i32)
        }}

        #[unsafe(no_mangle)]
        pub extern "C" fn get_{pm_name}_quick(
            interface: *const CInterfaceInstance
        ) -> {p_enum_name}_t {{
            let parameter_i32 = get_parameter_quick::<i32>(interface, ParameterId::{pm_id_name});
            FromPrimitive::from_i32(parameter_i32).unwrap_or_else(|| {{
                {p_enum_name}_t::default()
            }})
        }}
    "#)?;
            
    if !is_const {
        writeln!(f, r#"
            #[unsafe(no_mangle)]
            pub extern "C" fn set_{pm_name}(
                interface: *const CInterfaceInstance,
                {short_name}: {p_enum_name}_t,
                {short_name}_result: *mut {p_enum_name}_t
            ) -> EconfStatus {{
                let parameter_i32 = {short_name} as i32;
                let parameter_i32_result = {short_name}_result as *mut i32;
                set_parameter::<i32>(interface, ParameterId::{pm_id_name}, parameter_i32, parameter_i32_result)
            }}
        "#)?;
    }

    Ok(())
}

/// Converts C-style enum declarations with separate typedefs into combined typedef enum form
/// Example:
/// Input:  "enum CameraType_t { SOURCE_SIMULATOR = 0, SOURCE_CANON = 1 }; typedef int32_t CameraType_t;"
/// Output: "typedef enum { SOURCE_SIMULATOR = 0, SOURCE_CANON = 1 } CameraType_t;"
pub fn convert_enum_declarations(input: &str) -> String {
    // First pass: Find all enum declarations and their names
    let enum_decl_re = Regex::new(r"(?s)enum\s+(\w+)\s*\{(.*?)\}\s*;").unwrap();

    let mut result = input.to_string();
    
    // Find all enum declarations and collect their names
    let enum_names: Vec<String> = enum_decl_re.captures_iter(input)
        .map(|cap| cap[1].to_string())
        .collect();

    // For each enum name, find and convert matching typedefs
    for enum_name in enum_names {
        // Find the enum declaration
        if let Some(enum_cap) = enum_decl_re.captures(&result) {
            let enum_body = &enum_cap[2];
            
            // Find the corresponding typedef
            let typedef_pattern = format!(r"typedef\s+\w+\s+{}\s*;", enum_name);
            let typedef_re = Regex::new(&typedef_pattern).unwrap();
            
            if let Some(typedef_match) = typedef_re.find(&result) {
                // Replace both with combined form
                let replacement = format!("typedef enum {{\n  {}\n}} {};", enum_body.trim(), enum_name);
                let range = enum_cap.get(0).unwrap().start()..typedef_match.end();
                result.replace_range(range, &replacement);
            }
        }
    }
    
    result
}

pub(crate) fn process_convert_c_file(input_path: &Path, output_path: &Path) -> std::io::Result<()> {
    let content = fs::read_to_string(input_path)?;
    let converted = convert_enum_declarations(&content);
    let mut output_file = fs::File::create(output_path)?;
    output_file.write_all(converted.as_bytes())?;
    
    Ok(())
}
