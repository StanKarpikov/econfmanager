pub mod schema;
pub mod notifier;
pub mod arguments;
pub mod interface;
pub mod constants;
pub mod configfile;
pub mod database_utils;
pub mod event_receiver;
pub mod lib_helper_functions;
pub mod services {
    include!(concat!(env!("OUT_DIR"), "/", env!("SERVICE_PROTO_FILE_RS")));
}
pub mod parameter_ids {
    include!(concat!(env!("OUT_DIR"), "/", env!("PARAMETER_IDS_PROTO_FILE_RS")));
}
pub mod parameters {
    include!(concat!(env!("OUT_DIR"), "/parameters.rs"));
}
#[path = "../target/debug/parameter_functions.rs"] pub mod parameter_functions;

use timer::Timer;
use std::{ffi::{c_char, CString}, ptr, sync::{Arc, Mutex}};
use interface::{generated::ParameterId, InterfaceInstance, ParameterUpdateCallback};

#[repr(C)]
pub enum EconfStatus {
    StatusOk = 0,
    StatusError = 1
}

#[repr(C)]
#[derive (Clone)]
pub struct CInterfaceInstance(*mut Arc<Mutex<InterfaceInstance>>);

unsafe impl Send for CInterfaceInstance {}

impl CInterfaceInstance {
    pub(crate) fn new(state: InterfaceInstance) -> Self {
        let boxed_arc = Box::new(Arc::new(Mutex::new(state)));
        CInterfaceInstance(Box::into_raw(boxed_arc))
    }
    
    pub(crate) fn with_lock<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Mutex<InterfaceInstance>) -> R,
    {
        unsafe {
            if self.0.is_null() {
                panic!("Null pointer in CInterfaceInstance");
            }
            let arc = &*self.0;
            f(&arc)
        }
    }
    
    #[allow(unused)]
    pub(crate) fn with_lock_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut InterfaceInstance) -> R,
    {
        unsafe {
            if self.0.is_null() {
                panic!("Null pointer in CInterfaceInstance");
            }
            let arc = &*self.0;  // Immutable borrow of Arc
            let mut guard = arc.lock().unwrap();  // Lock the Mutex
            f(&mut guard)
        }
    }
    
    #[allow(unused)]
    pub(crate) fn get_arc(&self) -> Arc<Mutex<InterfaceInstance>> {
        unsafe {
            if self.0.is_null() {
                panic!("Null pointer in CInterfaceInstance");
            }
            (*self.0).clone()
        }
    }
}

impl Drop for CInterfaceInstance {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = Box::from_raw(self.0);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_init(
        database_path: *const std::os::raw::c_char,
        interface: *mut CInterfaceInstance
    ) -> EconfStatus {
    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };

    let r_instance = match InterfaceInstance::new(database_path) {
        Ok(value) => value,
        Err(_) => return EconfStatus::StatusError,
    };

    let c_instance = CInterfaceInstance::new(r_instance);

    unsafe { *interface = c_instance};

    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get_name(interface: CInterfaceInstance, id: ParameterId, name: *mut c_char, max_length: usize) -> EconfStatus {
    interface.with_lock(|lock| {
        let interface = lock.lock().unwrap();
        let rust_string = interface.get_name(id);

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
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_add_callback(interface: CInterfaceInstance, id: ParameterId, callback: ParameterUpdateCallback) -> EconfStatus {
    interface.with_lock(|lock| {
        let mut interface = lock.lock().unwrap();
        match interface.add_callback(id, callback) {
            Ok(_) => {
                EconfStatus::StatusOk
            }
            Err(_) => EconfStatus::StatusError,
        }
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_delete_callback(interface: CInterfaceInstance, id: ParameterId) -> EconfStatus {
    interface.with_lock(|lock| {
        let mut interface = lock.lock().unwrap();
        match interface.delete_callback(id) {
            Ok(_) => {
                EconfStatus::StatusOk
            }
            Err(_) => EconfStatus::StatusError,
        }
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_update_poll(interface: CInterfaceInstance) -> EconfStatus {
    interface.with_lock(|lock| {
        let mut interface = lock.lock().unwrap();
        match interface.update() {
            Ok(_) => {
                EconfStatus::StatusOk
            }
            Err(_) => EconfStatus::StatusError,
        }
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_set_up_timer_poll(interface: CInterfaceInstance, timer_period_ms: i64) -> EconfStatus {
    interface.with_lock(|lock| {
        let mut interface_guard = lock.lock().unwrap();

        let timer = Timer::new();
        
        let arc_clone = (unsafe { &*interface.0 }).clone();
        interface_guard.poll_timer_guard = Some(timer.schedule_repeating(chrono::Duration::milliseconds(timer_period_ms), move || {
            let mut interface = arc_clone.lock().unwrap();
            let _ = interface.update();
        }));

        EconfStatus::StatusOk
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_stop_timer_poll(interface: CInterfaceInstance) -> EconfStatus {
    interface.with_lock(|lock| {
        let mut interface = lock.lock().unwrap();
        drop(interface.poll_timer_guard.clone().unwrap());
        interface.poll_timer_guard = None;
    });
    EconfStatus::StatusOk
}
