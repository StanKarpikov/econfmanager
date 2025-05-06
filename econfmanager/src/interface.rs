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

use generated::{ParameterId, PARAMETERS_NUM, PARAMETER_DATA, GROUPS_DATA};
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
        default_data_folder: &String, 
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::new(database_path, saved_database_path, default_data_folder)?;
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
    
    pub fn get_groups(&self) -> Vec<(String,String, String)> {
        GROUPS_DATA.iter().map(|group| (group.name.to_string(), group.title.to_string(), group.comment.to_string())).collect()
    }

    pub fn get_group(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.split("@").next().unwrap().to_string()
    }

    pub fn get_name(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.to_owned()
    }

    pub fn get_comment(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].comment.to_owned()
    }

    pub fn get_is_const(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].is_const
    }

    pub fn get_runtime(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].runtime
    }

    pub fn get_type_string(&self, id: ParameterId) -> String {
        match &PARAMETER_DATA[id as usize].value_type {
            ParameterValue::ValBool(_) => "Bool".to_owned(),
            ParameterValue::ValI32(_) => "I32".to_owned(),
            ParameterValue::ValU32(_) => "U32".to_owned(),
            ParameterValue::ValI64(_) => "I64".to_owned(),
            ParameterValue::ValU64(_) => "U64".to_owned(),
            ParameterValue::ValF32(_) => "F32".to_owned(),
            ParameterValue::ValF64(_) => "F64".to_owned(),
            ParameterValue::ValString(_) => "String".to_owned(),
            ParameterValue::ValBlob(_) => "Blob".to_owned(),
            ParameterValue::ValPath(_) => todo!(),
        }
    }

    pub fn get_title(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].title.to_owned()
    }

    pub fn value_to_string(value: &ParameterValue) -> String {
        match value {
            ParameterValue::ValBool(b) => b.to_string(),
            ParameterValue::ValI32(i) => i.to_string(),
            ParameterValue::ValU32(u) => u.to_string(),
            ParameterValue::ValI64(i) => i.to_string(),
            ParameterValue::ValU64(u) => u.to_string(),
            ParameterValue::ValF32(f) => f.to_string(),
            ParameterValue::ValF64(f) => f.to_string(),
            ParameterValue::ValString(s) => s.to_string(),
            ParameterValue::ValBlob(data) => BASE64_STANDARD.encode(data),
            ParameterValue::ValPath(_) => todo!(),
        }
    }

    pub fn set_from_string(&self, id: ParameterId, value: &str) -> Result<ParameterValue> {
        let param_type = &PARAMETER_DATA[id as usize].value_type;
        
        let converted_value = match param_type {
            ParameterValue::ValBool(_) => {
                        match value.to_lowercase().as_str() {
                            "true" | "1" => ParameterValue::ValBool(true),
                            "false" | "0" => ParameterValue::ValBool(false),
                            _ => return Err(anyhow!("Expected 'true' or 'false' for boolean")),
                        }
                    },
            ParameterValue::ValI32(_) => {
                        value.parse::<i32>()
                            .map(ParameterValue::ValI32)
                            .map_err(|_| anyhow!("Expected a 32-bit integer"))?
                    },
            ParameterValue::ValU32(_) => {
                        value.parse::<u32>()
                            .map(ParameterValue::ValU32)
                            .map_err(|_| anyhow!("Expected an unsigned 32-bit integer"))?
                    },
            ParameterValue::ValI64(_) => {
                        value.parse::<i64>()
                            .map(ParameterValue::ValI64)
                            .map_err(|_| anyhow!("Expected a 64-bit integer"))?
                    },
            ParameterValue::ValU64(_) => {
                        value.parse::<u64>()
                            .map(ParameterValue::ValU64)
                            .map_err(|_| anyhow!("Expected an unsigned 64-bit integer"))?
                    },
            ParameterValue::ValF32(_) => {
                        value.parse::<f32>()
                            .map(ParameterValue::ValF32)
                            .map_err(|_| anyhow!("Expected a 32-bit float"))?
                    },
            ParameterValue::ValF64(_) => {
                        value.parse::<f64>()
                            .map(ParameterValue::ValF64)
                            .map_err(|_| anyhow!("Expected a 64-bit float"))?
                    },
            ParameterValue::ValString(_) => {
                        ParameterValue::ValString(value.to_string().into())
                    },
            ParameterValue::ValBlob(_) => {
                        let decoded = BASE64_STANDARD.decode(value)?;
                        ParameterValue::ValBlob(decoded)
                    }
            ParameterValue::ValPath(_) => todo!(),
        };
        
        Ok(converted_value)
    }

    pub fn set_from_json(&self, id: ParameterId, value: &Value) -> Result<ParameterValue> {
        let param_type = &PARAMETER_DATA[id as usize].value_type;
    
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
                                .map(|v| ParameterValue::ValString(v.to_string().into()))
                                .ok_or_else(|| anyhow!("Expected a string"))?,
            ParameterValue::ValBlob(_) => {
                                let base64_str = value.as_str().ok_or_else(|| anyhow!("Expected a base64 string"))?;
                                let decoded = BASE64_STANDARD.decode(base64_str)?;
                                ParameterValue::ValBlob(decoded)
                            }
            ParameterValue::ValPath(_) => todo!(),
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
        self.database.load_database()?;
        self.database.update()
    }

    pub fn factory_reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.drop_database()?;
        self.database.update()
    }
    
    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let filter = |key: &String| {
            PARAMETER_DATA
                .iter()
                .enumerate()
                .find(|(_, parameter)| parameter.name_id.to_string() == *key)
                .and_then(|(id, _)| Some(!PARAMETER_DATA[id].runtime))
                .unwrap_or(false)
        };
        self.database.save_database(&filter)
    }

}



