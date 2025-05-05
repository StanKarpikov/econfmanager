use std::{borrow::Cow, error::Error, fmt, mem};
use prost_reflect::{DescriptorPool, DynamicMessage, FileDescriptor, MessageDescriptor, ReflectMessage, Value};
use serde::ser::{Serialize, Serializer};

pub(crate) struct SchemaManager {
    config_descriptor: MessageDescriptor,
    file_descriptor: FileDescriptor,
}

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub enum ParameterValue {
    ValBool(bool),
    ValI32(i32),
    ValU32(u32),
    ValI64(i64),
    ValU64(u64),
    ValF32(f32),
    ValF64(f64),
    ValString(Cow<'static, str>),
    ValBlob(Vec<u8>),
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
            ParameterValue::ValBlob(v) => v.serialize(serializer),
            // ParameterValue::ValStaticString(v) => v.serialize(serializer),
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
            // ParameterValue::ValStaticString(v) => write!(f, "String: {}", v),
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
pub enum ValidationMethod {
    None,           // Default: no validation
    Range {
        min: ParameterValue,
        max: ParameterValue,
    },
    AllowedValues {
        values: Cow<'static, [ParameterValue]>
    },
    CustomCallback, // Validate using a callback function
}

#[repr(C)]
pub struct Parameter {
    pub value: ParameterValue,
    pub name_id: &'static str,
    pub validation: ValidationMethod,
    pub comment: &'static str,
    pub is_const: bool,
    pub tags: Vec<&'static str>,
    pub runtime: bool,
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
            .ok_or("Configuration message not found in descriptor pool")?;
        
        let file_descriptor = pool.get_file_by_name(&proto_name)
        .ok_or(format!("{} file descriptor not found", proto_name))?;

        Ok(Self { config_descriptor, file_descriptor })
    }

    fn convert_to_parameter_value(value: &Value) -> Option<ParameterValue> {
        let (_, value) = value.as_message().unwrap().fields().next().unwrap();
        match value {
            Value::I32(v) => Some(ParameterValue::ValI32(*v)),
            Value::U32(v) => Some(ParameterValue::ValU32(*v)),
            Value::F32(v) => Some(ParameterValue::ValF32(*v)),
            Value::String(v) => Some(ParameterValue::ValString(v.clone().into())),
            Value::Message(msg) => {
                Some(ParameterValue::ValBlob(vec![]))
            },
            _ => None
        }
    }

    pub(crate) fn get_parameters(&self) -> Result<Vec<Parameter>, Box<dyn Error>> {
        let default_config = DynamicMessage::new(self.config_descriptor.clone());
        let mut parameters = Vec::new();
        for field in default_config.descriptor().fields() {
            let value = &*default_config.get_field(&field);
            match value {
                Value::Message(nested_msg) => {
                    for pm_field in nested_msg.descriptor().fields() {
                        let field_type = pm_field.kind();
                        let mut parameter = Parameter{ 
                            value: match field_type {
                                prost_reflect::Kind::Double => ParameterValue::ValF64(0.0),
                                prost_reflect::Kind::Float => ParameterValue::ValF32(0.0),
                                prost_reflect::Kind::Int32 => ParameterValue::ValI32(0),
                                prost_reflect::Kind::Int64 => ParameterValue::ValI64(0),
                                prost_reflect::Kind::Uint32 => ParameterValue::ValU32(0),
                                prost_reflect::Kind::Uint64 => ParameterValue::ValU64(0), 
                                prost_reflect::Kind::Bool => ParameterValue::ValBool(false),
                                prost_reflect::Kind::String => ParameterValue::ValString(Cow::Borrowed("")),
                                prost_reflect::Kind::Bytes => ParameterValue::ValBlob(vec![]),
                                prost_reflect::Kind::Message(_) => {
                                    // For message types, we'll treat them as blobs
                                    ParameterValue::ValBlob(vec![])
                                },
                                prost_reflect::Kind::Enum(enum_descriptor) => ParameterValue::ValI32(0),
                                _ => ParameterValue::ValI32(0), //todo!()
                            },
                            // NOTE: Leak is okay since this function is only called at build time
                            name_id: Box::leak(Box::new(format!("{}@{}", field.name().to_string(), pm_field.name().to_string()))), 
                            validation: ValidationMethod::None, 
                            comment: "", 
                            is_const: false,
                            tags: Vec::new(),
                            runtime: false, 
                        };

                        let field_options = pm_field.options();

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

                        let default_value = field_options.extensions()
                            .find(|(desc, _)| desc.name() == "default_value")
                            .and_then(|(_, val)| Self::convert_to_parameter_value(val));

                        if let Some(default_value) = default_value {
                            if mem::discriminant(&parameter.value) != mem::discriminant(&default_value)
                            {
                                return Err(format!("Field {} default value {} is of the wrong type, expected {}", parameter.name_id, default_value, parameter.value).into());
                            }
                            parameter.value = default_value;
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
                                            min: ParameterValue::ValI32(0), // Placeholder
                                            max: ParameterValue::ValI32(0)  // Placeholder
                                        }
                                    },
                                    2 => {
                                        ValidationMethod::AllowedValues { values: Cow::Borrowed(&[]) } // Placeholder
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
                                    .and_then(|(_, val)| Self::convert_to_parameter_value(val))
                                    .ok_or(format!("Error: Range validation requires 'min' option for {}. Options: {}", parameter.name_id, field_options))?;
                                
                                *max = field_options.extensions()
                                    .find(|(desc, _)| desc.name() == "max")
                                    .and_then(|(_, val)| Self::convert_to_parameter_value(val))
                                    .ok_or(format!("Error: Range validation requires 'max' option for {}. Options: {}", parameter.name_id, field_options))?;
                                
                                if mem::discriminant(&parameter.value) != mem::discriminant(&max)
                                {
                                    return Err(format!("Field {} max value {} is of the wrong type, expected {}", parameter.name_id, max, parameter.value).into());
                                }

                                if mem::discriminant(&parameter.value) != mem::discriminant(&min)
                                {
                                    return Err(format!("Field {} min value {} is of the wrong type, expected {}", parameter.name_id, min, parameter.value).into());
                                }

                                if field_options.extensions().any(|(desc, _)| desc.name() == "allowed_values") {
                                    eprintln!("Warning: allowed_values ignored for Range validation for {}. Options: {}", parameter.name_id, field_options);
                                }
                            },
                            
                            ValidationMethod::AllowedValues { values } => {
                                *values = field_options.extensions()
                                    .find(|(desc, _)| desc.name() == "allowed_values")
                                    .and_then(|(_, val)| {
                                        if let Value::List(list) = val {
                                            Some(list.iter().filter_map(Self::convert_to_parameter_value).collect())
                                        } else {
                                            None
                                        }
                                    })
                                    .ok_or(format!("Error: AllowedValues validation requires 'allowed_values' option {}. Options: {}", parameter.name_id, field_options))?;
                                
                                for value in values.iter() {
                                    if mem::discriminant(&parameter.value) != mem::discriminant(&value)
                                    {
                                        return Err(format!("Field {} one of the allowed values {} is of the wrong type, expected {}", parameter.name_id, value, parameter.value).into());
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
        Ok(parameters)
    }

}
