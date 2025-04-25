pub mod schema;
pub mod arguments;
pub mod interface;
pub mod configfile;
pub mod database_utils;

use interface::{generated::Parameters, get, get_name, init};
use schema::{Parameter, SchemaManager};

#[repr(C)]
pub enum Status {
    StatusOk,
    StatusError
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_init(
        database_path: *const std::os::raw::c_char
    ) -> Status {
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    match init(database_path) {
        Ok(_) => Status::StatusOk,
        Err(_) => Status::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get(id: Parameters) -> &'static Parameter {
    get(id)
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get_name(id: Parameters) -> *const u8 {
    get_name(id)
}