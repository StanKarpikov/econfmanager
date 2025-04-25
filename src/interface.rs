use std::error::Error;

use crate::event_receiver::EventReceiver;
use crate::notifier::Notifier;
use crate::{configfile::Config, schema::Parameter};
use crate::database_utils::{DatabaseManager, Status};
use crate::schema::ParameterValue;

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{ParameterId, PARAMETERS_NUM, PARAMETER_DATA};

pub(crate) struct InterfaceInstance {
    database: DatabaseManager,
    notifier: Notifier,
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
        let _ = EventReceiver::new()?;
        Ok(Self{database, notifier})
    }
    
    pub(crate) fn get(&self, id: ParameterId) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        self.database.read_or_create(id)
    }
    
    pub(crate) fn set(&self, id: ParameterId, parameter: ParameterValue) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let result = self.database.write(id, parameter, false);
        match result {
            Ok(status) => match status {
                Status::StatusOkChanged(value) | 
                Status::StatusOkNotChecked(value) |
                Status::StatusOkOverflowFixed(value) => {
                    self.notifier.notify_of_parameter_change(id)?;
                    Ok(value)
                }
                Status::StatusOkNotChanged(value) => Ok(value),
                Status::StatusErrorNotAccepted(_) => Err("Parameter not accepted".into()),
                Status::StatusErrorFailed => Err("Failed to write the parameter".into()),
            },
            Err(_) => Err("Failed to write in the database".into()),
        }
    }
    
    pub(crate) fn get_name(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.to_owned()
    }

}



