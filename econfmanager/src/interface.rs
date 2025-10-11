use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Result, anyhow};
use base64::prelude::*;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use serde_json::Value;

use crate::config::Config;
use crate::database_utils::{DatabaseManager, Status};
use crate::event_receiver::EventReceiver;
use crate::generated;
use crate::notifier::Notifier;
use crate::schema::{ParameterValue, ParameterValueType};

use generated::{GROUPS_DATA, PARAMETER_DATA, PARAMETERS_NUM, ParameterId};

pub type ParameterUpdateCallback = Arc<dyn Fn(ParameterId) + Send + Sync + 'static>;

#[derive(Default)]
pub(crate) struct RuntimeParametersData {
    pub(crate) value: Option<ParameterValue>,
    pub(crate) callback: Option<ParameterUpdateCallback>,
}

pub(crate) struct SharedRuntimeData {
    pub(crate) parameters_data: [RuntimeParametersData; PARAMETERS_NUM],
}

impl SharedRuntimeData {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let parameters_data = std::array::from_fn(|_| RuntimeParametersData {
            value: None,
            callback: None,
        });
        Ok(Self { parameters_data })
    }
}

impl Default for SharedRuntimeData {
    fn default() -> Self {
        Self {
            parameters_data: std::array::from_fn(|_| RuntimeParametersData::default()),
        }
    }
}

#[derive(Default)]
pub struct InterfaceInstance {
    database: Arc<Mutex<DatabaseManager>>,
    notifier: Notifier,
    runtime_data: Arc<Mutex<SharedRuntimeData>>,
    event_receiver: Arc<Mutex<EventReceiver>>,
    timer_thread: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl InterfaceInstance {
    pub fn new(
        database_path: &String,
        saved_database_path: &String,
        default_data_folder: &String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::new(database_path, saved_database_path, default_data_folder)?;
        let database = Arc::new(Mutex::new(DatabaseManager::new(&config)?));
        let runtime_data = Arc::new(Mutex::new(SharedRuntimeData::new()?));
        let notifier = Notifier::new()?;
        let event_receiver = Arc::new(Mutex::new(EventReceiver::new(runtime_data.clone())?));
        info!(
            "Interface created: {} {}",
            &config.database_path, &config.saved_database_path
        );
        Ok(Self {
            database,
            notifier,
            runtime_data,
            event_receiver,
            timer_thread: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn get(
        &self,
        id: ParameterId,
        force: bool,
    ) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        let mut data = self.runtime_data.lock().unwrap();
        if !force && data.parameters_data[index].value.is_some() {
            let value = data.parameters_data[index].value.clone().unwrap();
            debug!(
                "Get parameter {}:[{}] from cache: {}",
                index, PARAMETER_DATA[index].name_id, value
            );
            return Ok(value);
        } else {
            let value = self.database.lock().unwrap().read_or_create(id)?;
            debug!(
                "Get parameter {}:[{}]: {}",
                index, PARAMETER_DATA[index].name_id, value
            );
            data.parameters_data[index].value = Some(value.clone());
            Ok(value)
        }
    }

    pub fn set(
        &self,
        id: ParameterId,
        parameter: ParameterValue,
    ) -> Result<ParameterValue, Box<dyn std::error::Error>> {
        let index: usize = id as usize;
        if PARAMETER_DATA[index].is_const {
            return Err(format!("Parameter {index} is const. Setting denied").into());
        }
        let result = self.database.lock().unwrap().write(id, parameter, false);
        let value = match result {
            Ok(status) => match status {
                Status::StatusOkChanged(value)
                | Status::StatusOkNotChecked(value)
                | Status::StatusOkOverflowFixed(value) => {
                    debug!(
                        "Set parameter {}:[{}]: {}",
                        index, PARAMETER_DATA[index].name_id, value
                    );
                    self.notifier.notify_of_parameter_change(id)?;
                    value
                }
                Status::StatusOkNotChanged(value) => {
                    debug!(
                        "Parameter {}:[{}] not changed",
                        index, PARAMETER_DATA[index].name_id
                    );
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

    pub fn get_groups(&self) -> Vec<(String, String, String)> {
        GROUPS_DATA
            .iter()
            .map(|group| {
                (
                    group.name.to_string(),
                    group.title.to_string(),
                    group.comment.to_string(),
                )
            })
            .collect()
    }

    pub fn get_group(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize]
            .name_id
            .split("@")
            .next()
            .unwrap()
            .to_string()
    }

    pub fn get_name(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].name_id.to_owned()
    }

    pub fn get_comment(&self, id: ParameterId) -> String {
        PARAMETER_DATA[id as usize].comment.to_owned()
    }

    pub fn is_const(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].is_const
    }

    pub fn is_runtime(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].runtime
    }

    pub fn is_readonly(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].readonly
    }

    pub fn is_internal(&self, id: ParameterId) -> bool {
        PARAMETER_DATA[id as usize].internal
    }

    pub fn get_tags(&self, id: ParameterId) -> Vec<String> {
        PARAMETER_DATA[id as usize].tags.iter().map(|val|val.to_string()).collect()
    }
    
    pub fn get_validation_json(&self, id: ParameterId) -> serde_json::Value {
        match &PARAMETER_DATA[id as usize].validation {
            crate::schema::ValidationMethod::None => serde_json::json!("none"),
            crate::schema::ValidationMethod::Range { min, max } => {
                serde_json::json!({
                    "range": {
                        "min": Self::value_to_string(&min),
                        "max": Self::value_to_string(&max)
                    }
                })
            },
            crate::schema::ValidationMethod::AllowedValues { values, names } => {
                let values_iter = values.iter();
                let names_iter = names.iter();
                let value_pairs: Vec<_> = values_iter
                    .zip(names_iter)
                    .map(|(value, name)| {
                        match value {
                            ParameterValue::ValEnum(_) =>
                                serde_json::json!({
                                    "value": Self::value_to_string(value),
                                    "name": name
                                }),
                            _ =>
                            serde_json::json!({
                                "value": Self::value_to_string(value),
                                "name": Self::value_to_string(value),
                            }),
                        }
                    })
                    .collect();
                serde_json::json!({ "allowed_values": value_pairs })
            },
            crate::schema::ValidationMethod::CustomCallback => serde_json::json!("custom"),
        }
    }
    
    pub fn get_type_string(&self, id: ParameterId) -> String {
        match &PARAMETER_DATA[id as usize].value_type {
            ParameterValueType::TypeBool => "Bool".to_owned(),
            ParameterValueType::TypeI32 => "I32".to_owned(),
            ParameterValueType::TypeU32 => "U32".to_owned(),
            ParameterValueType::TypeI64 => "I64".to_owned(),
            ParameterValueType::TypeU64 => "U64".to_owned(),
            ParameterValueType::TypeF32 => "F32".to_owned(),
            ParameterValueType::TypeF64 => "F64".to_owned(),
            ParameterValueType::TypeString => "String".to_owned(),
            ParameterValueType::TypeBlob => "Blob".to_owned(),
            ParameterValueType::TypeEnum(_) => "I32".to_owned(),
            ParameterValueType::TypeNone => "None".to_owned(),
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
            ParameterValue::ValNone => todo!(),
            ParameterValue::ValEnum(i) => i.to_string(),
        }
    }

    pub fn set_from_string(&self, id: ParameterId, value: &str) -> Result<ParameterValue> {
        let param_type = &PARAMETER_DATA[id as usize].value_type;

        let converted_value = match param_type {
            ParameterValueType::TypeBool => match value.to_lowercase().as_str() {
                        "true" | "1" => ParameterValue::ValBool(true),
                        "false" | "0" => ParameterValue::ValBool(false),
                        _ => return Err(anyhow!("Expected 'true' or 'false' for boolean")),
                    },
            ParameterValueType::TypeI32 => value
                        .parse::<i32>()
                        .map(ParameterValue::ValI32)
                        .map_err(|_| anyhow!("Expected a 32-bit integer"))?,
            ParameterValueType::TypeU32 => value
                        .parse::<u32>()
                        .map(ParameterValue::ValU32)
                        .map_err(|_| anyhow!("Expected an unsigned 32-bit integer"))?,
            ParameterValueType::TypeI64 => value
                        .parse::<i64>()
                        .map(ParameterValue::ValI64)
                        .map_err(|_| anyhow!("Expected a 64-bit integer"))?,
            ParameterValueType::TypeU64 => value
                        .parse::<u64>()
                        .map(ParameterValue::ValU64)
                        .map_err(|_| anyhow!("Expected an unsigned 64-bit integer"))?,
            ParameterValueType::TypeF32 => value
                        .parse::<f32>()
                        .map(ParameterValue::ValF32)
                        .map_err(|_| anyhow!("Expected a 32-bit float"))?,
            ParameterValueType::TypeF64 => value
                        .parse::<f64>()
                        .map(ParameterValue::ValF64)
                        .map_err(|_| anyhow!("Expected a 64-bit float"))?,
            ParameterValueType::TypeString => ParameterValue::ValString(value.to_string().into()),
            ParameterValueType::TypeBlob => {
                        let decoded = BASE64_STANDARD.decode(value)?;
                        ParameterValue::ValBlob(decoded)
                    }
            ParameterValueType::TypeEnum(_) => value
                        .parse::<i32>()
                        .map(ParameterValue::ValEnum)
                        .map_err(|_| anyhow!("Expected a 32-bit integer"))?,
            ParameterValueType::TypeNone => ParameterValue::ValNone,
        };

        Ok(converted_value)
    }

    pub fn set_from_json(&self, id: ParameterId, value: &Value) -> Result<ParameterValue> {
        let param_type = &PARAMETER_DATA[id as usize].value_type;

        let converted_value = match param_type {
            ParameterValueType::TypeBool => value
                        .as_bool()
                        .map(ParameterValue::ValBool)
                        .ok_or_else(|| anyhow!("Expected a boolean"))?,
            ParameterValueType::TypeI32 => value
                        .as_i64()
                        .map(|v| ParameterValue::ValI32(v as i32))
                        .ok_or_else(|| anyhow!("Expected an integer"))?,
            ParameterValueType::TypeU32 => value
                        .as_u64()
                        .map(|v| ParameterValue::ValU32(v as u32))
                        .ok_or_else(|| anyhow!("Expected an unsigned integer"))?,
            ParameterValueType::TypeI64 => value
                        .as_i64()
                        .map(ParameterValue::ValI64)
                        .ok_or_else(|| anyhow!("Expected an integer"))?,
            ParameterValueType::TypeU64 => value
                        .as_u64()
                        .map(ParameterValue::ValU64)
                        .ok_or_else(|| anyhow!("Expected an unsigned integer"))?,
            ParameterValueType::TypeF32 => value
                        .as_f64()
                        .map(|v| ParameterValue::ValF32(v as f32))
                        .ok_or_else(|| anyhow!("Expected a float"))?,
            ParameterValueType::TypeF64 => value
                        .as_f64()
                        .map(ParameterValue::ValF64)
                        .ok_or_else(|| anyhow!("Expected a float"))?,
            ParameterValueType::TypeString => value
                        .as_str()
                        .map(|v| ParameterValue::ValString(v.to_string().into()))
                        .ok_or_else(|| anyhow!("Expected a string"))?,
            ParameterValueType::TypeBlob => {
                        let base64_str = value
                            .as_str()
                            .ok_or_else(|| anyhow!("Expected a base64 string"))?;
                        let decoded = BASE64_STANDARD.decode(base64_str)?;
                        ParameterValue::ValBlob(decoded)
                    }
            ParameterValueType::TypeEnum(_) => value
                        .as_i64()
                        .map(|v| ParameterValue::ValEnum(v as i32))
                        .ok_or_else(|| anyhow!("Expected an integer"))?,
            ParameterValueType::TypeNone => ParameterValue::ValNone,
        };

        Ok(converted_value)
    }

    pub fn get_parameter_names(&self) -> Vec<String> {
        PARAMETER_DATA
            .iter()
            .map(|parameter| parameter.name_id.to_string())
            .collect()
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

    pub fn update(&mut self) -> Result<Vec<ParameterId>, Box<dyn std::error::Error>> {
        info!("Update called");
        let pending_callbacks = self.database.lock().unwrap().update()?;
        for id in &pending_callbacks {
            self.event_receiver.lock().unwrap().notify_callback(*id);
        }
        Ok(pending_callbacks)
    }

    pub fn start_periodic_update(&mut self, interval: Duration) {
        self.stop_periodic_update();

        let stop_flag = Arc::new(AtomicBool::new(false));
        self.stop_flag = stop_flag.clone();

        let shared_database = self.database.clone();
        let shared_event_receiver = self.event_receiver.clone();
        
        let handle = thread::spawn(move || {
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                let pending_callbacks = 
                {
                    debug!("Timer update");
                    let mut database = shared_database.lock().unwrap();
                    database.update()
                };

                match pending_callbacks {
                    Ok(pending_callbacks) =>
                        for id in &pending_callbacks {
                            shared_event_receiver.lock().unwrap().notify_callback(*id);
                        },
                    Err(e) => error!("Timer update failed: {}", e)
                }

                thread::sleep(interval);
            }
        });

        self.timer_thread = Some(handle);
    }

    pub fn stop_periodic_update(&mut self) {
        if let Some(flag) = Arc::get_mut(&mut self.stop_flag) {
            flag.store(true, Ordering::Relaxed);
        }
        
        if let Some(handle) = self.timer_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn add_callback(
        &mut self,
        id: ParameterId,
        callback: ParameterUpdateCallback,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

    pub fn notify_all_force(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for id in 0..PARAMETER_DATA.len() {
            self.notifier.notify_of_parameter_change(ParameterId::try_from(id)?)?;
        }
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.lock().unwrap().load_database()?;
        self.notify_all_force()
    }

    pub fn factory_reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.database.lock().unwrap().drop_database()?;
        self.notify_all_force()
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let filter = |key: &String| {
            PARAMETER_DATA
                .iter()
                .enumerate()
                .find(|(_, parameter)| parameter.name_id.to_string() == *key)
                .and_then(|(id, _)| {
                    let to_save = !PARAMETER_DATA[id].runtime;
                    if to_save {
                        info!("Saving parameter {}", key);
                    }
                    else {
                        info!("Skipping runtime parameter {}", key);
                    }
                    Some(to_save)
                })
                .unwrap_or(false)
        };
        self.database.lock().unwrap().save_database(&filter)
    }
}

impl Drop for InterfaceInstance {
    fn drop(&mut self) {
        self.stop_periodic_update();
    }
}