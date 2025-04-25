use crate::{configfile::Config, schema::Parameter};
use crate::database_utils::DatabaseManager;
use crate::schema::SchemaManager;

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{Parameters, PARAMETER_DATA};

pub(crate) fn init(
    database_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let descriptors_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/descriptors.bin"));
    let config = Config::new(env!("CONFIGURATION_PROTO_FILE").to_string(), database_path)?;
    let schema = SchemaManager::new("".to_owned(), descriptors_bytes.to_vec(), config.proto_name.clone())?;
    let database = DatabaseManager::new(config)?;

    // schema.prepare_database(database)?;

    println!("Database created successfully!");
    Ok(())
}

pub(crate) fn get(id: Parameters) -> &'static Parameter {
    let _ = id;
    let parameter = &PARAMETER_DATA[4];
    parameter
}

pub(crate) fn get_name(id: Parameters) -> *const u8 {
    let _ = id;
    PARAMETER_DATA[3].name_id.as_ptr()
}