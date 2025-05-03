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

use std::io::Write;
use std::time::Duration;
use lib_helper_functions::interface_execute;
use log::error;
use timer::Timer;
use log::LevelFilter;
use log::info;
use parking_lot::Mutex;
use std::{ffi::{c_char, CString}, ptr, sync::Arc};
use interface::{generated::ParameterId, InterfaceInstance, ParameterUpdateCallback};

const LOCK_TRYING_DURATION: Duration = Duration::from_secs(1);

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
    
    pub(crate) fn with_lock<F, R>(&self, f: F) -> Result<R, Box<dyn std::error::Error>>
    where
        F: FnOnce(&Mutex<InterfaceInstance>) -> R,
    {
        if self.0.is_null() {
            error!("Null pointer in CInterfaceInstance");
            return Err("Null pointer in CInterfaceInstance".into());
        }
        let arc = unsafe {&*self.0};
        Ok(f(&arc))
    }
    
    #[allow(unused)]
    pub(crate) fn with_lock_mut<F, R>(&self, f: F) -> Result<R, Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut InterfaceInstance) -> R,
    {
        if self.0.is_null() {
            error!("Null pointer in CInterfaceInstance");
            return Err("Null pointer in CInterfaceInstance".into());
        }
        let arc = unsafe {&*self.0};  // Immutable borrow of Arc
        let mut guard = arc.lock();  // Lock the Mutex
        Ok(f(&mut guard))
    }
    
    #[allow(unused)]
    pub(crate) fn get_arc(&self) -> Result<Arc<Mutex<InterfaceInstance>>, Box<dyn std::error::Error>> {
        if self.0.is_null() {
            error!("Null pointer in CInterfaceInstance");
            return Err("Null pointer in CInterfaceInstance".into());
        }
        Ok(unsafe { (*self.0).clone() })
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
        saved_database_path: *const std::os::raw::c_char,
        interface: *mut *mut CInterfaceInstance
    ) -> EconfStatus {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter_level(LevelFilter::Debug)
        .init();

    let database_path = unsafe { std::ffi::CStr::from_ptr(database_path).to_string_lossy().into_owned() };
    let saved_database_path = unsafe { std::ffi::CStr::from_ptr(saved_database_path).to_string_lossy().into_owned() };

    let r_instance = match InterfaceInstance::new(&database_path, &saved_database_path) {
        Ok(value) => value,
        Err(_) => return EconfStatus::StatusError,
    };

    let c_instance = CInterfaceInstance::new(r_instance);

    unsafe { *interface = Box::into_raw(Box::new(c_instance))};

    info!("Initialisation done: database_path={} saved_database_path={}", database_path, saved_database_path);
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_get_name(interface: *const CInterfaceInstance, id: ParameterId, name: *mut c_char, max_length: usize) -> EconfStatus {
    interface_execute(interface, |interface| {
        let rust_string = interface.get_name(id);

        let c_string = match CString::new(rust_string) {
            Ok(s) => s,
            Err(e) => return Err(Box::new(e)),
        };

        let bytes = c_string.as_bytes_with_nul();
        
        if bytes.len() > max_length {
            return Err("Max length exceeded".into());
        }

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, name, bytes.len());
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_add_callback(interface: *const CInterfaceInstance, id: ParameterId, callback: ParameterUpdateCallback) -> EconfStatus {
    interface_execute(interface, |interface| {
        interface.add_callback(id, callback)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_delete_callback(interface: *const CInterfaceInstance, id: ParameterId) -> EconfStatus {
    interface_execute(interface, |interface| {
        interface.delete_callback(id)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_update_poll(interface: *const CInterfaceInstance) -> EconfStatus {
    interface_execute(interface, |interface| {
        interface.update()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_set_up_timer_poll(interface: *const CInterfaceInstance, timer_period_ms: i64) -> EconfStatus {
    let arc_interface = match unsafe { &*interface }.get_arc(){
        Ok(value) => value,
        Err(_) => return EconfStatus::StatusError
    };
    interface_execute(interface, |interface_guard| {
        let timer = Timer::new();
        interface_guard.poll_timer_guard = Some(timer.schedule_repeating(
            chrono::Duration::milliseconds(timer_period_ms),
            move || {
                let mut guard = match arc_interface.try_lock_for(LOCK_TRYING_DURATION) {
                    Some(g) => g,
                    None => {
                        error!("Failed to acquire lock in timer callback");
                        return;
                    }
                };

                if let Err(e) = guard.update() {
                    error!("Timer update failed: {}", e);
                }
            },
        ));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_stop_timer_poll(interface: *const CInterfaceInstance) -> EconfStatus {
    interface_execute(interface, |interface| {
        if let Some(guard) = interface.poll_timer_guard.take() {
            drop(guard);
            Ok(())
        } else {
            Err("No active timer to stop".into())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_load(interface: *const CInterfaceInstance) -> EconfStatus {
    interface_execute(interface, |interface| {
        interface.load()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_save(interface: *const CInterfaceInstance) -> EconfStatus {
    interface_execute(interface, |interface| {
        interface.save()
    })
}
