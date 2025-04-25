use crate::{configfile::Config, schema::Parameter};
use crate::database_utils::DatabaseManager;
use crate::schema::{ParameterValue, SchemaManager};

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{Parameters, PARAMETER_DATA};

pub struct InterfaceInstance {
    database: DatabaseManager
}

impl InterfaceInstance {
    pub(crate) fn new(
        database_path: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // let descriptors_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/descriptors.bin"));
        let config = Config::new(env!("CONFIGURATION_PROTO_FILE").to_string(), database_path)?;
        // let schema = SchemaManager::new("".to_owned(), descriptors_bytes.to_vec(), config.proto_name.clone())?;
        let database = DatabaseManager::new(config)?;
        Ok(Self{database})
    }
    
    pub(crate) fn get(&self, id: Parameters) -> &'static ParameterValue {
        self.database.read_or_create(id).unwrap()
    }
    
    pub(crate) fn set(&self, id: Parameters, parameter: ParameterValue) -> &'static ParameterValue {
        self.database.write(id, parameter)
    }
    
    pub(crate) fn get_name(&self, id: Parameters) -> *const u8 {
        PARAMETER_DATA[id as usize].name_id.as_ptr()
    } 
}



