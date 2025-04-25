use std::error::Error;

use crate::notifier::Notifier;
use crate::{configfile::Config, schema::Parameter};
use crate::database_utils::{DatabaseManager, Status};
use crate::schema::ParameterValue;

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{ParameterId, PARAMETER_DATA};

pub struct InterfaceInstance {
    database: DatabaseManager,
    notifier: Notifier
}

impl InterfaceInstance {
    pub(crate) fn new(
        database_path: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // let descriptors_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/descriptors.bin"));
        let config = Config::new(env!("CONFIGURATION_PROTO_FILE").to_string(), database_path)?;
        // let schema = SchemaManager::new("".to_owned(), descriptors_bytes.to_vec(), config.proto_name.clone())?;
        let database = DatabaseManager::new(config)?;
        let notifier = Notifier::new()?;
        Ok(Self{database, notifier})
    }
    
    pub(crate) fn get(&self, id: ParameterId) -> &'static ParameterValue {
        // self.database.read_or_create(id).unwrap()
        &ParameterValue::ValI32(0)
    }
    
    pub(crate) fn set(&self, id: ParameterId, parameter: ParameterValue) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let result = self.database.write(id, parameter, false);
        match result {
            Ok(status) => match status {
                Status::StatusOkChanged(value) | 
                Status::StatusOkNotChecked(value) |
                Status::StatusOkOverflowFixed(value) => {
                    self.notifier.notify_of_parameter_change(id); Ok(value)
                }
                Status::StatusOkNotChanged(value) => Ok(value),
                Status::StatusErrorNotAccepted(_) => Err("Parameter no accepted".into()),
                Status::StatusErrorFailed => Err("Failed to write the parameter".into()),
            },
            Err(_) => Err("Failed to write in the database".into()),
        }
    }
    
    pub(crate) fn get_name(&self, id: ParameterId) -> *const u8 {
        PARAMETER_DATA[id as usize].name_id.as_ptr()
    } 
}



