pub mod schema;
pub mod notifier;
pub mod arguments;
pub mod interface;
pub mod configfile;
pub mod database_utils;
pub mod event_receiver;

use std::{ffi::{c_char, CString}, ptr};

use interface::generated::ParameterId;
use schema::ParameterValue;

#[repr(C)]
pub enum EconfStatus {
    StatusOk = 0,
    StatusError = 1
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_init(
        database_path: *const std::os::raw::c_char
    ) -> EconfStatus {
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    // match init(database_path) {
    //     Ok(_) => Status::StatusOk,
    //     Err(_) => Status::StatusError,
    // }
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get(id: ParameterId, out_value: *mut ParameterValue) -> EconfStatus {
    // get(id)
    unsafe { *out_value = ParameterValue::ValI32(0)};
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get_name(id: ParameterId, name: *mut c_char, max_length: usize) -> EconfStatus {
    // get_name(id)

    let rust_string = "";

    // Create a C-compatible string
    let c_string = match CString::new(rust_string) {
        Ok(s) => s,
        Err(_) => return EconfStatus::StatusError,
    };

    let bytes = c_string.as_bytes_with_nul();
    
    if bytes.len() > max_length {
        return EconfStatus::StatusError;
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, name, bytes.len());
    }
    EconfStatus::StatusOk
}