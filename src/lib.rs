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