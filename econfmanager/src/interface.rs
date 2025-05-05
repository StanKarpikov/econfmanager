use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use log::{debug, info, warn, error};
use serde_json::Value;
use base64::prelude::*;
use anyhow::{anyhow, Result};

use crate::config::Config;
use crate::event_receiver::EventReceiver;
use crate::generated;
use crate::notifier::Notifier;
use crate::database_utils::{DatabaseManager, Status};
use crate::schema::ParameterValue;

use generated::{ParameterId, PARAMETERS_NUM, PARAMETER_DATA};
use timer::Guard;

pub type ParameterUpdateCallback = Arc<dyn Fn(ParameterId) + Send + Sync + 'static>;

#[derive(Default)]
pub(crate) struct RuntimeParametersData {
    pub(crate) value: Option<ParameterValue>,
    pub(crate) callback: Option<ParameterUpdateCallback>
}

#[derive(Default)]
pub(crate) struct SharedRuntimeData {
    pub(crate) parameters_data: [RuntimeParametersData; PARAMETERS_NUM],
}

impl SharedRuntimeData{
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let parameters_data= std::array::from_fn(|_| RuntimeParametersData { value: None, callback: None });
        Ok(Self{parameters_data})
    }
}

#[derive(Default)]
pub struct InterfaceInstance {
    database: DatabaseManager,
    notifier: Notifier,
    runtime_data: Arc<Mutex<SharedRuntimeData>>,
    pub(crate) poll_timer_guard: Option<Guard>
}

impl InterfaceInstance {
    pub fn new(
        database_path: &String,
        saved_database_path: &String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::new(database_path, saved_database_path)?;
        let database = DatabaseManager::new(&config)?;
        let runtime_data = Arc::new(Mutex::new(SharedRuntimeData::new()?));
        let notifier = Notifier::new()?;
        let _ = EventReceiver::new(runtime_data.clone())?;
        info!("Interface created: {} {}", &config.database_path, &config.saved_database_path);
        Ok(Self{database, notifier, runtime_data, poll_timer_guard:None})
    }
    
    pub fn get(&self, id: ParameterId, force: bool) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        let mut data = self.runtime_data.lock().unwrap();
        if !force && data.parameters_data[index].value.is_some() {
            let value = data.parameters_data[index].value.clone().unwrap();
            debug!("Get parameter {}:[{}] from cache: {}", index, PARAMETER_DATA[index].name_id, value);
            return Ok(value);
        }
        else {
            let value = self.database.read_or_create(id)?;
            debug!("Get parameter {}:[{}]: {}", index, PARAMETER_DATA[index].name_id, value);
            data.parameters_data[index].value = Some(value.clone());
            Ok(value)
        }
    }
    
    pub fn set(&self, id: ParameterId, parameter: ParameterValue) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        let result = self.database.write(id, parameter, false);
        let value = match result {
            Ok(status) => match status {
                Status::StatusOkChanged(value) | 
                Status::StatusOkNotChecked(value) |
                Status::StatusOkOverflowFixed(value) => {
                    debug!("Set parameter {}:[{}]: {}", index, PARAMETER_DATA[index].name_id, value);
                    self.notifier.notify_of_parameter_change(id)?;
                    value
                }
                Status::StatusOkNotChanged(value) => {
                    debug!("Parameter {}:[{}] not changed", index, PARAMETER_DATA[index].name_id);
                    value
                }
                Status::StatusErrorNotAccepted(_) => return Err("Parameter not accepted".into()),
                Status::StatusErrorFailed => return Err("Failed to write the parameter".into()),
            },
            Err(e) => return Err(format!("Failed to write in the database: {}", e).into()),
        };

        let mut data = self.runtime_data.lock().unwrap();
        data.parameters_data[index].value = Some(value.clone());
        Ok(value)
    }
    
    pub fn get_name(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.to_owned()
    }

    pub fn set_from_json(&self, id: ParameterId, value: &Value) -> Result<ParameterValue> {
        let param_type = &PARAMETER_DATA[id as usize].value;
    
        let converted_value = match param_type {
            ParameterValue::ValBool(_) => value
                .as_bool()
                .map(ParameterValue::ValBool)
                .ok_or_else(|| anyhow!("Expected a boolean"))?,
    
            ParameterValue::ValI32(_) => value
                .as_i64()
                .map(|v| ParameterValue::ValI32(v as i32))
                .ok_or_else(|| anyhow!("Expected an integer"))?,
    
            ParameterValue::ValU32(_) => value
                .as_u64()
                .map(|v| ParameterValue::ValU32(v as u32))
                .ok_or_else(|| anyhow!("Expected an unsigned integer"))?,
    
            ParameterValue::ValI64(_) => value
                .as_i64()
                .map(ParameterValue::ValI64)
                .ok_or_else(|| anyhow!("Expected an integer"))?,
    
            ParameterValue::ValU64(_) => value
                .as_u64()
                .map(ParameterValue::ValU64)
                .ok_or_else(|| anyhow!("Expected an unsigned integer"))?,
    
            ParameterValue::ValF32(_) => value
                .as_f64()
                .map(|v| ParameterValue::ValF32(v as f32))
                .ok_or_else(|| anyhow!("Expected a float"))?,
    
            ParameterValue::ValF64(_) => value
                .as_f64()
                .map(ParameterValue::ValF64)
                .ok_or_else(|| anyhow!("Expected a float"))?,
    
            ParameterValue::ValString(_) => value
                .as_str()
                .map(|v| ParameterValue::ValString(v.to_string()))
                .ok_or_else(|| anyhow!("Expected a string"))?,
    
            ParameterValue::ValBlob(_) => {
                let base64_str = value.as_str().ok_or_else(|| anyhow!("Expected a base64 string"))?;
                let decoded = BASE64_STANDARD.decode(base64_str)?;
                ParameterValue::ValBlob(decoded)
            }
        };
    
        Ok(converted_value)
    }

    pub fn get_parameter_names(&self) -> Vec<String> {
        PARAMETER_DATA.iter().map(|parameter| parameter.name_id.to_string()).collect()
    }

    pub fn get_parameters_number(&self) -> usize {
        PARAMETER_DATA.len()
    }
    
    pub fn get_parameter_id_from_name(&self, name: String) -> Option<ParameterId> {
        PARAMETER_DATA
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.name_id.to_string() == name)
            .and_then(|(id, _)| ParameterId::try_from(id).ok())
    }

    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.update()
    }

    pub fn add_callback(&mut self, id: ParameterId, callback: ParameterUpdateCallback) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            {
                let mut data = self.runtime_data.lock().unwrap();
                data.parameters_data[index].callback = Some(callback);
                info!("Callback added for ID {}", index);
            }
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

    pub fn delete_callback(&mut self, id: ParameterId) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            {
                let mut data = self.runtime_data.lock().unwrap();
                data.parameters_data[index].callback = None;
                info!("Callback removed for ID {}", index);
            }
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.load_database()
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.save_database()
    }

}



