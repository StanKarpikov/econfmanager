use std::error::Error;
use prost_reflect::{DescriptorPool, DynamicMessage, FileDescriptor, MessageDescriptor, ReflectMessage, Value};


// use crate::configfile::Config;
// use crate::database_utils::DatabaseManager;


pub(crate) struct SchemaManager {
    config_descriptor: MessageDescriptor,
    file_descriptor: FileDescriptor,
}

#[repr(C)]
#[derive(Clone, PartialEq)]
pub enum ParameterValue {
    ValBool(bool),
    ValI32(i32),
    ValU32(u32),
    ValI64(i64),
    ValU64(u64),
    ValF32(f32),
    ValF64(f64),
    ValString(String),
    ValBlob(Vec<u8>),
}

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
}

// Implement for all basic types
impl_parameter_type!(bool, ValBool);
impl_parameter_type!(i32, ValI32);
impl_parameter_type!(u32, ValU32);
impl_parameter_type!(i64, ValI64);
impl_parameter_type!(u64, ValU64);
impl_parameter_type!(f32, ValF32);
impl_parameter_type!(f64, ValF64);
impl_parameter_type!(String, ValString);
impl_parameter_type!(Vec<u8>, ValBlob);

#[repr(C)]
pub enum ValidationMethod {
    None,           // Default: no validation
    Range {
        min: ParameterValue,
        max: ParameterValue,
    },
    AllowedValues {
        values: Vec<ParameterValue>
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
    pub tags: Vec<&'static str>
}

impl SchemaManager {

    /******************************************************************************
     * PRIVATE FUNCTIONS
     ******************************************************************************/

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

    // pub(crate) fn prepare_database(&self, mut db: DatabaseManager) -> Result<(), Box<dyn Error>> {
    //     db.set_sqlite_version(self.get_required_version()?)?;
    
    //     let parameters = self.get_parameters()?;

    //     db.insert_parameters(parameters)?;
        
    //     Ok(())
    // }

    pub(crate) fn get_parameters(&self) -> Result<Vec<Parameter>, Box<dyn Error>> {
        let default_config = DynamicMessage::new(self.config_descriptor.clone());
        let mut parameters = Vec::new();
        for field in default_config.descriptor().fields() {
            let value = &*default_config.get_field(&field);
            match value {
                Value::Message(nested_msg) => {
                    for pm_field in nested_msg.descriptor().fields() {
                        let field_type = pm_field.kind();
                        let parameter = Parameter{ 
                            value: match field_type {
                                prost_reflect::Kind::Double => ParameterValue::ValF64(0.0),
                                prost_reflect::Kind::Float => ParameterValue::ValF32(0.0),
                                prost_reflect::Kind::Int32 => ParameterValue::ValI32(0),
                                prost_reflect::Kind::Int64 => ParameterValue::ValI32(0),
                                prost_reflect::Kind::Uint32 => ParameterValue::ValI32(0),
                                prost_reflect::Kind::Uint64 => ParameterValue::ValI32(0), 
                                prost_reflect::Kind::Bool => ParameterValue::ValI32(0),
                                prost_reflect::Kind::String => ParameterValue::ValI32(0),
                                prost_reflect::Kind::Bytes => ParameterValue::ValI32(0),
                                // prost_reflect::Kind::Message(message_descriptor) => todo!(),
                                prost_reflect::Kind::Enum(enum_descriptor) => ParameterValue::ValI32(0),
                                _ => ParameterValue::ValI32(0), //todo!()
                            },
                            // NOTE: Leak is okay since this function is only called at build time
                            name_id: Box::leak(Box::new(format!("{}@{}", field.name().to_string(), pm_field.name().to_string()))), 
                            validation: ValidationMethod::None, 
                            comment: "", 
                            is_const: false,
                            tags: Vec::new() 
                        };
                        
                        // if let Some(opts) = field.proto().options.as_ref() {
                        //     if opts.has_extension(options::default_value) {
                        //         // TODO: Check the type
                        //         parameter.value = opts.get_extension(options::default_value);
                        //     }
                        //     if opts.has_extension(options::comment) {
                        //         parameter.comment = opts.get_extension(options::comment);
                        //     }
                        //     if opts.has_extension(options::is_const) {
                        //         parameter.is_const = opts.get_extension(options::is_const);
                        //     }
                        //     if opts.has_extension(options::tags) {
                        //         parameter.tags = opts.get_extension(options::tags);
                        //     }
                        //     if opts.has_extension(options::validation) {
                        //         parameter.validation = opts.get_extension(options::validation);
                        //     }
                        // }
                        
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
