use std::error::Error;
use prost_reflect::{DescriptorPool, DynamicMessage, FileDescriptor, MessageDescriptor, Value};

use crate::configfile::Config;
use crate::database_utils::DatabaseManager;

pub(crate) struct SchemaManager {
    config_descriptor: MessageDescriptor,
    file_descriptor: FileDescriptor,
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
    
    pub(crate) fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let descriptor_path = std::path::Path::new(&config.descriptors_path);

        let descriptor_bytes = std::fs::read(descriptor_path)?;
        let pool = DescriptorPool::decode(&*descriptor_bytes)?;
    
        let config_descriptor = pool.get_message_by_name("Configuration")
            .ok_or("Configuration message not found in descriptor pool")?;
        
        let file_descriptor = pool.get_file_by_name(&config.proto_name)
        .ok_or("configuration.proto file descriptor not found")?;

        Ok(Self { config_descriptor, file_descriptor })
    }

    pub(crate) fn prepare_database(&self, mut db: DatabaseManager) -> Result<(), Box<dyn Error>> {
        let default_config = DynamicMessage::new(self.config_descriptor.clone());

        db.set_sqlite_version(self.get_required_version()?)?;
    
        // Recursively insert all fields
        db.process_config(
            &default_config,
            ""
        )?;
        
        Ok(())
    }

}
