pub mod schema;
pub mod arguments;
pub mod configfile;
pub mod database_utils;

use schema::{AnyValue, Parameter, SchemaManager, ValidationMethod};
use configfile::Config;
use database_utils::DatabaseManager;

#[path = "../target/debug/generated.rs"] pub mod generated;
use generated::{Parameters, PARAMETER_DATA};

#[repr(C)]
pub enum Status {
    StatusOk,
    StatusError
}

/******************************************************************************
 * PRIVATE FUNCTIONS
 ******************************************************************************/

fn init_internal(
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

/******************************************************************************
 * PUBLIC FUNCTIONS
 ******************************************************************************/

#[unsafe(no_mangle)]
pub extern "C" fn init(
        database_path: *const std::os::raw::c_char
    ) -> Status {
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    match init_internal(database_path) {
        Ok(_) => Status::StatusOk,
        Err(_) => Status::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get(id: Parameters) -> &'static Parameter {
    let _ = id;
    let parameter = &PARAMETER_DATA[4];
    parameter
}

#[unsafe(no_mangle)]
pub extern "C" fn get_name(id: Parameters) -> &'static str {
    &PARAMETER_DATA[3].name_id
}