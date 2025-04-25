pub mod schema;
pub mod notifier;
pub mod arguments;
pub mod interface;
pub mod configfile;
pub mod database_utils;
pub mod event_receiver;
pub mod services {
    include!(concat!(env!("OUT_DIR"), "/", env!("SERVICE_PROTO_FILE_RS")));
}
pub mod parameter_ids {
    include!(concat!(env!("OUT_DIR"), "/", env!("PARAMETER_IDS_PROTO_FILE_RS")));
}
pub mod parameters {
    include!(concat!(env!("OUT_DIR"), "/", env!("parameters.rs")));
}
#[path = "../target/debug/parameter_functions.rs"] pub mod parameter_functions;

use std::{ffi::{c_char, CString}, ptr};

use interface::{generated::ParameterId, InterfaceInstance};
use schema::ParameterValue;

#[repr(C)]
pub enum EconfStatus {
    StatusOk = 0,
    StatusError = 1
}

#[repr(C)]
pub struct CInterfaceInstance(*mut InterfaceInstance);

impl CInterfaceInstance {
    fn new(state: InterfaceInstance) -> Self {
        CInterfaceInstance(Box::into_raw(Box::new(state)))
    }
    
    fn as_ref(&self) -> &InterfaceInstance {
        unsafe { &*self.0 }
    }
    
    fn as_mut(&mut self) -> &mut InterfaceInstance {
        unsafe { &mut *self.0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_init(
        database_path: *const std::os::raw::c_char,
        instance: *mut CInterfaceInstance
    ) -> EconfStatus {
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    let r_instance = match InterfaceInstance::new(database_path) {
        Ok(value) => value,
        Err(_) => return EconfStatus::StatusError,
    };

    let c_instance = CInterfaceInstance::new(r_instance);

    unsafe { *instance = c_instance};

    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get(instance: CInterfaceInstance, id: ParameterId, out_value: *mut ParameterValue) -> EconfStatus {
    let instance = instance.as_ref();
    match instance.get(id) {
        Ok(parameter) => {
            unsafe { *out_value = parameter.clone()}
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_set(instance: CInterfaceInstance, id: ParameterId, out_value: *mut ParameterValue) -> EconfStatus {
    let instance = instance.as_ref();
    let parameter = unsafe { (*out_value).clone()};
    match instance.set(id, parameter) {
        Ok(parameter) => {
            unsafe { *out_value = parameter.clone()}
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get_name(instance: CInterfaceInstance, id: ParameterId, name: *mut c_char, max_length: usize) -> EconfStatus {
    let instance = instance.as_ref();
    let rust_string = instance.get_name(id);

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

#[unsafe(no_mangle)]
pub extern "C" fn get_i32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut i32) -> EconfStatus {
    let instance = instance.as_ref();
    match instance.get(id) {
        Ok(parameter) => {
            let ret_val = match parameter {
                ParameterValue::ValI32(value) => value,
                _ => return EconfStatus::StatusError
            };
            unsafe { *out_parameter = ret_val.clone()};
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}

pub extern "C" fn set_i32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut i32) -> EconfStatus {
    let instance = instance.as_ref();
    let parameter = unsafe { (*out_parameter).clone()};
    match instance.set(id, schema::ParameterValue::ValI32(parameter)) {
        Ok(parameter) => {
            let ret_val = match parameter {
                ParameterValue::ValI32(value) => value,
                _ => return EconfStatus::StatusError
            };
            unsafe { *out_parameter = ret_val.clone()};
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_f32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut f32) -> EconfStatus {
    let instance = instance.as_ref();
    match instance.get(id) {
        Ok(parameter) => {
            let ret_val = match parameter {
                ParameterValue::ValF32(value) => value,
                _ => return EconfStatus::StatusError
            };
            unsafe { *out_parameter = ret_val.clone()};
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}

pub extern "C" fn set_f32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut f32) -> EconfStatus {
    let instance = instance.as_ref();
    let parameter = unsafe { (*out_parameter).clone()};
    match instance.set(id, schema::ParameterValue::ValF32(parameter)) {
        Ok(parameter) => {
            let ret_val = match parameter {
                ParameterValue::ValF32(value) => value,
                _ => return EconfStatus::StatusError
            };
            unsafe { *out_parameter = ret_val.clone()};
            EconfStatus::StatusOk
        },
        Err(_) => EconfStatus::StatusError,
    }
}