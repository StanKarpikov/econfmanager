pub mod schema;
pub mod arguments;
pub mod configfile;
pub mod database_utils;

use schema::{AnyValue, Parameter, SchemaManager, ValidationMethod};
use configfile::Config;
use database_utils::DatabaseManager;

#[path = "../target/debug/generated.rs"]  pub mod generated;
use generated::{Parameters, PARAMETER_ID};

#[repr(C)]
pub enum Status {
    StatusOk,
    StatusError
}

/******************************************************************************
 * PRIVATE FUNCTIONS
 ******************************************************************************/

fn init_internal(
    descriptors_path: String,
    proto_name: String,
    database_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(descriptors_path, proto_name, database_path)?;
    let schema = SchemaManager::new(config.descriptors_path.clone(), config.proto_name.clone())?;
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
        descriptors_path: *const std::os::raw::c_char,
        proto_name: *const std::os::raw::c_char,
        database_path: *const std::os::raw::c_char
    ) -> Status {
    let descriptors_path = unsafe { std::ffi::CStr::from_ptr(descriptors_path).to_string_lossy().into_owned() };
    let proto_name = unsafe { std::ffi::CStr::from_ptr(proto_name).to_string_lossy().into_owned() };
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    match init_internal(descriptors_path, proto_name, database_path) {
        Ok(_) => Status::StatusOk,
        Err(_) => Status::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get(id: Parameters) -> Parameter {
    let _ = id;
    let parameter = Parameter{ 
        value: AnyValue::ValI32(0),
        name_id: "".to_owned(), 
        validation: ValidationMethod::None, 
        comment: "".to_owned(), 
        is_const: false,
        tags: Vec::new() 
    };
    parameter
}

#[unsafe(no_mangle)]
pub extern "C" fn get_name(id: Parameters) -> String {
    PARAMETER_ID[3].to_owned()
}