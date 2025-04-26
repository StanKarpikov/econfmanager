use std::sync::{Arc, Mutex};

use crate::event_receiver::EventReceiver;
use crate::notifier::Notifier;
use crate::{configfile::Config, schema::Parameter};
use crate::database_utils::{DatabaseManager, Status};
use crate::schema::ParameterValue;

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{ParameterId, PARAMETERS_NUM, PARAMETER_DATA};
use timer::Guard;

pub type ParameterUpdateCallback = extern fn(id: ParameterId);

pub(crate) struct RuntimeParametersData {
    pub(crate) value: Option<ParameterValue>,
    pub(crate) callback: Option<ParameterUpdateCallback>
}

pub(crate) struct SharedRuntimeData {
    pub(crate) parameters_data: [RuntimeParametersData; PARAMETERS_NUM],
}

impl SharedRuntimeData{
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let parameters_data= std::array::from_fn(|_| RuntimeParametersData { value: None, callback: None });
        Ok(Self{parameters_data})
    }
}

pub(crate) struct InterfaceInstance {
    database: DatabaseManager,
    notifier: Notifier,
    runtime_data: Arc<Mutex<SharedRuntimeData>>,
    pub(crate) poll_timer_guard: Option<Guard>
}

impl InterfaceInstance {
    pub(crate) fn new(
        database_path: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::new(env!("CONFIGURATION_PROTO_FILE").to_string(), database_path)?;
        let database = DatabaseManager::new(config)?;
        let runtime_data = Arc::new(Mutex::new(SharedRuntimeData::new()?));
        let notifier = Notifier::new()?;
        let _ = EventReceiver::new(runtime_data.clone())?;
        Ok(Self{database, notifier, runtime_data, poll_timer_guard:None })
    }
    
    pub(crate) fn get(&self, id: ParameterId, force: bool) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        let mut data = self.runtime_data.lock().unwrap();
        if !force && data.parameters_data[index].value.is_some() {
            return Ok(data.parameters_data[index].value.clone().unwrap());
        }
        else {
            let value = self.database.read_or_create(id)?;
            data.parameters_data[index].value = Some(value.clone());
            Ok(value)
        }
    }
    
    pub(crate) fn set(&self, id: ParameterId, parameter: ParameterValue) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        let result = self.database.write(id, parameter, false);
        let value = match result {
            Ok(status) => match status {
                Status::StatusOkChanged(value) | 
                Status::StatusOkNotChecked(value) |
                Status::StatusOkOverflowFixed(value) => {
                    self.notifier.notify_of_parameter_change(id)?;
                    value
                }
                Status::StatusOkNotChanged(value) => value,
                Status::StatusErrorNotAccepted(_) => return Err("Parameter not accepted".into()),
                Status::StatusErrorFailed => return Err("Failed to write the parameter".into()),
            },
            Err(_) => return Err("Failed to write in the database".into()),
        };

        let mut data = self.runtime_data.lock().unwrap();
        data.parameters_data[index].value = Some(value.clone());
        Ok(value)
    }
    
    pub(crate) fn get_name(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.to_owned()
    }

    pub(crate) fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.update()
    }

    pub(crate) fn add_callback(&mut self, id: ParameterId, callback: ParameterUpdateCallback) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            {
                let mut data = self.runtime_data.lock().unwrap();
                data.parameters_data[index].callback = Some(callback);
            }
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

    pub(crate) fn delete_callback(&mut self, id: ParameterId) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            {
                let mut data = self.runtime_data.lock().unwrap();
                data.parameters_data[index].callback = None;
            }
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

}



