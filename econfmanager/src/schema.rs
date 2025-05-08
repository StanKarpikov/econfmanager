use std::{borrow::Cow, error::Error, fmt, mem};
use base64::{prelude::BASE64_STANDARD, Engine};
use prost_reflect::{DescriptorPool, DynamicMessage, FileDescriptor, MessageDescriptor, ReflectMessage, Value};
use serde::ser::{Serialize, Serializer};

pub(crate) struct SchemaManager {
    config_descriptor: MessageDescriptor,
    file_descriptor: FileDescriptor,
}

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub enum ParameterValue {
    ValNone,
    ValBool(bool),
    ValI32(i32),
    ValU32(u32),
    ValI64(i64),
    ValU64(u64),
    ValF32(f32),
    ValF64(f64),
    ValString(Cow<'static, str>),
    ValBlob(Vec<u8>),
    ValEnum(i32),
    ValPath(&'static str),
}

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub enum ParameterValueType {
    TypeNone,
    TypeBool,
    TypeI32,
    TypeU32,
    TypeI64,
    TypeU64,
    TypeF32,
    TypeF64,
    TypeString,
    TypeBlob,
    TypeEnum(Cow<'static, str>),
}

impl fmt::Display for ParameterValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValueType::TypeBool => write!(f, "Bool"),
            ParameterValueType::TypeI32 => write!(f, "I32"),
            ParameterValueType::TypeU32 => write!(f, "U32"),
            ParameterValueType::TypeI64 => write!(f, "I64"),
            ParameterValueType::TypeU64 => write!(f, "U64"),
            ParameterValueType::TypeF32 => write!(f, "F32"),
            ParameterValueType::TypeF64 => write!(f, "F64"),
            ParameterValueType::TypeString => write!(f, "String"),
            ParameterValueType::TypeBlob => write!(f, "Blob"),
            ParameterValueType::TypeEnum(v) => write!(f, "Enum: {}", v),
            ParameterValueType::TypeNone => write!(f, "None"),
        }
    }
}

impl ParameterValue {
    pub fn parameter_type(&self) -> ParameterValueType {
        match self {
            ParameterValue::ValNone => ParameterValueType::TypeNone,
            ParameterValue::ValBool(_) => ParameterValueType::TypeBool,
            ParameterValue::ValI32(_) => ParameterValueType::TypeI32,
            ParameterValue::ValU32(_) => ParameterValueType::TypeU32,
            ParameterValue::ValI64(_) => ParameterValueType::TypeI64,
            ParameterValue::ValU64(_) => ParameterValueType::TypeU64,
            ParameterValue::ValF32(_) => ParameterValueType::TypeF32,
            ParameterValue::ValF64(_) => ParameterValueType::TypeF64,
            ParameterValue::ValString(_) => ParameterValueType::TypeString,
            ParameterValue::ValBlob(_) => ParameterValueType::TypeBlob,
            ParameterValue::ValEnum(_) => ParameterValueType::TypeEnum(Cow::Borrowed("")),
            ParameterValue::ValPath(_) => ParameterValueType::TypeBlob,
        }
    }
}

impl Default for ParameterValue {
    fn default() -> Self {
        ParameterValue::ValI32(0)
    }
}

impl Serialize for ParameterValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ParameterValue::ValBool(v) => v.serialize(serializer),
            ParameterValue::ValI32(v) => v.serialize(serializer),
            ParameterValue::ValU32(v) => v.serialize(serializer),
            ParameterValue::ValI64(v) => v.serialize(serializer),
            ParameterValue::ValU64(v) => v.serialize(serializer),
            ParameterValue::ValF32(v) => v.serialize(serializer),
            ParameterValue::ValF64(v) => v.serialize(serializer),
            ParameterValue::ValString(v) => v.serialize(serializer),
            ParameterValue::ValBlob(v) => {
                        let encoded = BASE64_STANDARD.encode(v);
                        encoded.serialize(serializer)
                    },
            ParameterValue::ValEnum(v) => v.serialize(serializer),
            ParameterValue::ValPath(_) => todo!(),
            ParameterValue::ValNone => todo!(),
        }
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::ValBool(v) => write!(f, "Bool: {}", v),
            ParameterValue::ValI32(v) => write!(f, "I32: {}", v),
            ParameterValue::ValU32(v) => write!(f, "U32: {}", v),
            ParameterValue::ValI64(v) => write!(f, "I64: {}", v),
            ParameterValue::ValU64(v) => write!(f, "U64: {}", v),
            ParameterValue::ValF32(v) => write!(f, "F32: {:+.4e}", v),
            ParameterValue::ValF64(v) => write!(f, "F64: {:+.4e}", v),
            ParameterValue::ValString(v) => write!(f, "String: {}", v),
            ParameterValue::ValBlob(v) => {
                                                let display_len = std::cmp::min(8, v.len());
                                                write!(f, "[")?;
                                                for byte in &v[..display_len] {
                                                    write!(f, "{:02X} ", byte)?;
                                                }
                                                if v.len() > display_len {
                                                    write!(f, "... ({} bytes)", v.len())?;
                                                }
                                                write!(f, "]")
                                            }
            ParameterValue::ValPath(p) => write!(f, "Path: {}", p),
            ParameterValue::ValEnum(v) => write!(f, "Enum: {}", v),
            ParameterValue::ValNone => write!(f, "None"),
        }
    }
}

#[allow(unused)]
pub(crate) trait ParameterType: Clone {
    fn to_parameter_value(self) -> ParameterValue;
    fn from_parameter_value(value: ParameterValue) -> Option<Self>
    where
        Self: Sized;
}

macro_rules! impl_parameter_type {
    ($type:ty, $variant:ident) => {
        impl ParameterType for $type {
            fn to_parameter_value(self) -> ParameterValue {
                ParameterValue::$variant(self)
            }
            
            fn from_parameter_value(value: ParameterValue) -> Option<Self> {
                match value {
                    ParameterValue::$variant(v) => Some(v),
                    _ => None,
                }
            }
        }
    };
    
    (String => Cow, $variant:ident) => {
        impl ParameterType for String {
            fn to_parameter_value(self) -> ParameterValue {
                ParameterValue::$variant(Cow::Owned(self))
            }
            
            fn from_parameter_value(value: ParameterValue) -> Option<Self> {
                match value {
                    ParameterValue::$variant(v) => Some(v.into_owned()),
                    _ => None,
                }
            }
        }
    };

    (&str => Cow, $variant:ident) => {
        impl ParameterType for &'static str {
            fn to_parameter_value(self) -> ParameterValue {
                ParameterValue::$variant(Cow::Borrowed(self))
            }
            
            fn from_parameter_value(value: ParameterValue) -> Option<Self> {
                match value {
                    ParameterValue::$variant(Cow::Borrowed(v)) => Some(v),
                    _ => None,
                }
            }
        }
    };

    (c_char => Cow, $variant:ident) => {
        impl ParameterType for std::ffi::c_char {
            fn to_parameter_value(self) -> ParameterValue {
                let s = self.to_string();
                ParameterValue::$variant(Cow::Owned(s))
            }
            
            fn from_parameter_value(value: ParameterValue) -> Option<Self> {
                match value {
                    ParameterValue::$variant(v) => {
                        v.parse().ok()
                    },
                    _ => None,
                }
            }
        }
    };
}

impl_parameter_type!(bool, ValBool);
impl_parameter_type!(i32, ValI32);
impl_parameter_type!(u32, ValU32);
impl_parameter_type!(i64, ValI64);
impl_parameter_type!(u64, ValU64);
impl_parameter_type!(f32, ValF32);
impl_parameter_type!(f64, ValF64);
impl_parameter_type!(String => Cow, ValString);
impl_parameter_type!(&str => Cow, ValString);
impl_parameter_type!(c_char => Cow, ValString);
impl_parameter_type!(Vec<u8>, ValBlob);

#[repr(C)]
#[derive (Debug)]
pub enum ValidationMethod {
    None,           // Default: no validation
    Range {
        min: ParameterValue,
        max: ParameterValue,
    },
    AllowedValues {
        values: Cow<'static, [ParameterValue]>,
        names: Cow<'static, [&'static str]>,
    },
    CustomCallback, // Validate using a callback function
}

#[repr(C)]
#[derive (Debug)]
pub struct Parameter {
    pub value_type: ParameterValueType,
    pub value_default: ParameterValue,
    pub name_id: &'static str,
    pub validation: ValidationMethod,
    pub comment: &'static str,
    pub title: &'static str,
    pub is_const: bool,
    pub tags: Vec<&'static str>,
    pub runtime: bool,
}

#[repr(C)]
pub struct Group {
    pub name: &'static str,
    pub comment: &'static str,
    pub title: &'static str,
}

// This implementation is used during build time
#[allow(unused)]
impl SchemaManager {

    /******************************************************************************
     * PRIVATE FUNCTIONS
     ******************************************************************************/

    #[allow(unused)]
    fn get_required_version(&self) -> Result<u32, Box<dyn Error>> {
        let required_version = self.file_descriptor.options()
            .extensions()
            .find(|(ext, _)| ext.name() == "version")
            .and_then(|(_, value)| match &*value {
                Value::I32(v) => Some(*v as u32),
                Value::I64(v) => Some(*v as u32),
                Value::U32(v) => Some(*v),
                Value::U64(v) => Some(*v as u32),
                _ => None,
            })
            .ok_or("Version option not found or is not a valid integer type")?;    
        Ok(required_version)
    }

    /******************************************************************************
     * PUBLIC FUNCTIONS
     ******************************************************************************/
    
    pub(crate) fn new(descriptors_path: String, descriptor_bytes: Vec<u8>, proto_name: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut descriptor_bytes = descriptor_bytes;
        if descriptors_path.len() != 0 {
            let descriptor_path = std::path::Path::new(&descriptors_path);
            descriptor_bytes = std::fs::read(descriptor_path)?;
        }
        let pool = DescriptorPool::decode(&*descriptor_bytes)?;
    
        let config_descriptor = pool.get_message_by_name("parameters.Configuration")
            .ok_or("Configuration message 'parameters.Configuration' not found in descriptor pool. Check that the 'package parameters;' is defined in parameters.proto")?;
        
        let file_descriptor = pool.get_file_by_name(&proto_name)
        .ok_or(format!("{} file descriptor not found", proto_name))?;

        Ok(Self { config_descriptor, file_descriptor })
    }

    fn convert_to_parameter_value(value_type: &ParameterValueType, value: &Value) -> Option<ParameterValue> {
        let (_, value) = value.as_message().unwrap().fields().next().unwrap();
        match value {
            Value::Bool(v) => Some(ParameterValue::ValBool(*v)),
            Value::I32(v) =>  Some(ParameterValue::ValI32(*v)),
            Value::U32(v) => Some(ParameterValue::ValU32(*v)),
            Value::I64(v) => Some(ParameterValue::ValI64(*v)),
            Value::U64(v) => Some(ParameterValue::ValU64(*v)),
            Value::F32(v) => Some(ParameterValue::ValF32(*v)),
            Value::F64(v) => Some(ParameterValue::ValF64(*v)),
            Value::String(v) => 
                match value_type {
                    ParameterValueType::TypeString => Some(ParameterValue::ValString(v.clone().into())),
                    ParameterValueType::TypeBlob => Some(ParameterValue::ValPath(Box::leak(Box::new(v.clone())))),
                    _ => None
                },
            Value::Message(msg) => {
                match value_type {
                    ParameterValueType::TypeEnum(_) => {
                        if let Some((_, value)) = msg.fields().next() {
                            Some(ParameterValue::ValEnum(value.as_enum_number().unwrap()))
                        }
                        else {
                            todo!("Only custom_type oneof is supported at the moment");
                        }
                    },
                    _ => todo!("Blob parameters not supported at the moment"),
                }
            },
            Value::EnumNumber(enum_value) => {
                Some(ParameterValue::ValEnum(*enum_value as i32))
            },
            _ => todo!("Unknown type")
        }
    }

    pub(crate) fn get_parameters(&self) -> Result<(Vec<Parameter>, Vec<Group>), Box<dyn Error>> {
        let default_config = DynamicMessage::new(self.config_descriptor.clone());
        let mut groups = Vec::new();
        let mut parameters = Vec::new();
        for field in default_config.descriptor().fields() {
            let value = &*default_config.get_field(&field);
            match value {
                Value::Message(nested_msg) => {
                    
                    let group_options = field.options();
                    let mut group = Group {
                        title: Box::leak(Box::new(group_options.extensions()
                        .find(|(desc, _)| desc.name() == "title")
                        .and_then(|(_, val)| val.as_str())
                        .unwrap_or(field.name()).to_string())),

                        comment: Box::leak(Box::new(group_options.extensions()
                            .find(|(desc, _)| desc.name() == "comment")
                            .and_then(|(_, val)| val.as_str())
                            .unwrap_or("").to_string())),

                        name: Box::leak(Box::new(field.name().to_string()))
                    };
                    
                    groups.push(group);

                    for pm_field in nested_msg.descriptor().fields() {
                        let field_type = pm_field.kind();
                        let mut parameter = Parameter{ 
                            value_type: match field_type {
                                prost_reflect::Kind::Double => ParameterValueType::TypeF64,
                                prost_reflect::Kind::Float => ParameterValueType::TypeF32,
                                prost_reflect::Kind::Int32 => ParameterValueType::TypeI32,
                                prost_reflect::Kind::Int64 => ParameterValueType::TypeI64,
                                prost_reflect::Kind::Uint32 => ParameterValueType::TypeU32,
                                prost_reflect::Kind::Uint64 => ParameterValueType::TypeU64, 
                                prost_reflect::Kind::Bool => ParameterValueType::TypeBool,
                                prost_reflect::Kind::String => ParameterValueType::TypeString,
                                prost_reflect::Kind::Bytes => ParameterValueType::TypeBlob,
                                prost_reflect::Kind::Enum(enum_descriptor) => {
                                    ParameterValueType::TypeEnum(Cow::Owned(enum_descriptor.name().to_string()))
                                },
                                prost_reflect::Kind::Message(msg) => {
                                    // For other message types, we'll treat them as blobs
                                    ParameterValueType::TypeBlob
                                },
                                _ => todo!("Unsupported paramter kind {:?}", field_type)
                            },
                            value_default: ParameterValue::ValNone,
                            // NOTE: Leak is okay since this function is only called at build time
                            name_id: Box::leak(Box::new(format!("{}@{}", field.name().to_string(), pm_field.name().to_string()))), 
                            validation: ValidationMethod::None, 
                            comment: "", 
                            title: "",
                            is_const: false,
                            tags: Vec::new(),
                            runtime: false, 
                        };

                        let field_options = pm_field.options();

                        parameter.title = Box::leak(Box::new(field_options.extensions()
                            .find(|(desc, _)| desc.name() == "title")
                            .and_then(|(_, val)| val.as_str())
                            .unwrap_or(pm_field.name()).to_string()));

                        parameter.comment = Box::leak(Box::new(field_options.extensions()
                            .find(|(desc, _)| desc.name() == "comment")
                            .and_then(|(_, val)| val.as_str())
                            .unwrap_or("").to_string()));

                        parameter.runtime = field_options.extensions()
                            .find(|(desc, _)| desc.name() == "runtime")
                            .and_then(|(_, val)| val.as_bool())
                            .unwrap_or(false);

                        parameter.is_const = field_options.extensions()
                            .find(|(desc, _)| desc.name() == "is_const")
                            .and_then(|(_, val)| val.as_bool())
                            .unwrap_or(false);

                        let value_default = field_options.extensions()
                            .find(|(desc, _)| desc.name() == "default_value")
                            .and_then(|(_, val)| {
                                let val = Self::convert_to_parameter_value(&parameter.value_type, val);
                                if val.is_none() 
                                {
                                    panic!("Could not process default value for {}/{}", field.name().to_string(), pm_field.name().to_string());
                                }
                                val
                                });

                        if let Some(value_default) = value_default {
                            if mem::discriminant(&parameter.value_type) != mem::discriminant(&value_default.parameter_type())
                            {
                                return Err(format!("Field {} default value {} is of the wrong type, expected {}", parameter.name_id, value_default, parameter.value_type).into());
                            }
                            parameter.value_default = value_default;
                        }
                        else {
                            panic!("No default value found for {}/{}", field.name().to_string(), pm_field.name().to_string());
                        }

                        let validation = field_options.extensions()
                            .find(|(desc, _)| desc.name() == "validation");

                        if let Some((_, validation_value)) = validation {
                            let val = validation_value.as_enum_number();
                            if let Some(val_i32) = val {
                                parameter.validation = match val_i32 {
                                    0 => {
                                        ValidationMethod::None
                                    },
                                    1 => {
                                        ValidationMethod::Range {
                                            min: ParameterValue::ValNone, // Placeholder
                                            max: ParameterValue::ValNone  // Placeholder
                                        }
                                    },
                                    2 => {
                                        ValidationMethod::AllowedValues { values: Cow::Borrowed(&[]), names: Cow::Borrowed(&[]) } // Placeholder
                                    },
                                    3 => ValidationMethod::CustomCallback,
                                    _ => {
                                        ValidationMethod::None
                                    }
                                };
                            }
                            else {
                                eprintln!("Validation method has wrong type {:?} for {}", val, parameter.name_id);
                            }
                        }

                        // Force allowed values for Enum fields
                        if let prost_reflect::Kind::Enum(enum_desc) = pm_field.kind()
                        {
                            match &mut parameter.validation {
                                ValidationMethod::None => parameter.validation = ValidationMethod::AllowedValues { values: Cow::Borrowed(&[]), names: Cow::Borrowed(&[]) },
                                ValidationMethod::AllowedValues { values, names } => {},
                                _ => todo!("Only allowed values validation method is supported for enums"),
                            }
                        }

                        match &mut parameter.validation {
                            ValidationMethod::None => {
                                if field_options.extensions().any(|(desc, _)| 
                                    ["min", "max", "allowed_values"].contains(&desc.name())
                                ) {
                                    eprintln!("Warning: Validation options set but validation method is None for {}. Options: {}", parameter.name_id, field_options);
                                }
                            },
                            
                            ValidationMethod::Range { min, max } => {
                                *min = field_options.extensions()
                                    .find(|(desc, _)| desc.name() == "min")
                                    .and_then(|(_, val)| Self::convert_to_parameter_value(&parameter.value_type, val))
                                    .ok_or(format!("Error: Range validation requires 'min' option for {}. Options: {}", parameter.name_id, field_options))?;
                                
                                *max = field_options.extensions()
                                    .find(|(desc, _)| desc.name() == "max")
                                    .and_then(|(_, val)| Self::convert_to_parameter_value(&parameter.value_type, val))
                                    .ok_or(format!("Error: Range validation requires 'max' option for {}. Options: {}", parameter.name_id, field_options))?;
                                
                                if mem::discriminant(&parameter.value_type) != mem::discriminant(&max.parameter_type())
                                {
                                    return Err(format!("Field {} max value {} is of the wrong type, expected {}", parameter.name_id, max, parameter.value_type).into());
                                }

                                if mem::discriminant(&parameter.value_type) != mem::discriminant(&min.parameter_type())
                                {
                                    return Err(format!("Field {} min value {} is of the wrong type, expected {}", parameter.name_id, min, parameter.value_type).into());
                                }

                                if field_options.extensions().any(|(desc, _)| desc.name() == "allowed_values") {
                                    eprintln!("Warning: allowed_values ignored for Range validation for {}. Options: {}", parameter.name_id, field_options);
                                }
                            },
                            
                            ValidationMethod::AllowedValues { values, names} => {
                                if let prost_reflect::Kind::Enum(enum_desc) = pm_field.kind()
                                {
                                    *values = enum_desc.values().map(|v| ParameterValue::ValEnum(v.number())).collect();
                                    let mut names_str: Box<[&'static str]> = enum_desc.values().map(|v| Box::leak(v.name().to_string().into_boxed_str()) as &'static str).collect();
                                    *names = Cow::Owned(names_str.into_vec());
                                }
                                else {
                                    *values = field_options.extensions()
                                        .find(|(desc, _)| desc.name() == "allowed_values")
                                        .and_then(|(_, val)| {
                                            if let Value::List(list) = val {
                                                Some(list.iter().filter_map(|val| {Self::convert_to_parameter_value(&parameter.value_type, val)}).collect())
                                            } else {
                                                None
                                            }
                                        })
                                        .ok_or(format!("Error: AllowedValues validation requires 'allowed_values' option {}. Options: {}", parameter.name_id, field_options))?;
                                    
                                    for value in values.iter() {
                                        if mem::discriminant(&parameter.value_type) != mem::discriminant(&value.parameter_type())
                                        {
                                            return Err(format!("Field {} one of the allowed values {} is of the wrong type, expected {}", parameter.name_id, value, parameter.value_type).into());
                                        }
                                    }
                                }
    
                                if field_options.extensions().any(|(desc, _)| ["min", "max"].contains(&desc.name())) {
                                    eprintln!("Warning: min/max options ignored for AllowedValues validation {}. Options: {}", parameter.name_id, field_options);
                                }
                            },
                            
                            ValidationMethod::CustomCallback => {}
                        }

                        parameters.push(parameter);
                    }
                }
                _ => {
                    return Err(format!("Field {} will be ignored, the configuration requires two levels of definitions", field.name().to_string()).into());
                }
            }
        }
        Ok((parameters, groups))
    }

}
