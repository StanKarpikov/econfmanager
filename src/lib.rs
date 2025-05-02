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
            let mut guard = arc.lock();  // Lock the Mutex
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
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
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
pub extern "C" fn econf_add_callback(interface: *const CInterfaceInstance, id: ParameterId, callback: ParameterUpdateCallback) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
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
pub extern "C" fn econf_delete_callback(interface: *const CInterfaceInstance, id: ParameterId) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
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
pub extern "C" fn econf_update_poll(interface: *const CInterfaceInstance) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
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
pub extern "C" fn econf_set_up_timer_poll(interface: *const CInterfaceInstance, timer_period_ms: i64) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface_guard = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };

        let timer = Timer::new();
        
        let arc_clone = (unsafe { &*interface.0 }).clone();
        interface_guard.poll_timer_guard = Some(timer.schedule_repeating(chrono::Duration::milliseconds(timer_period_ms), move || {
            let mut interface = arc_clone.lock();
            let _ = interface.update();
        }));

        EconfStatus::StatusOk
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_stop_timer_poll(interface: *const CInterfaceInstance) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface_guard = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            },
        };
        
        // Take ownership of the timer guard and drop it
        if let Some(timer_guard) = interface_guard.poll_timer_guard.take() {
            drop(timer_guard);
        }
        
        EconfStatus::StatusOk
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_load(interface: *const CInterfaceInstance) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
        match interface.load() {
            Ok(_) => {
                EconfStatus::StatusOk
            }
            Err(_) => EconfStatus::StatusError,
        }
    });
    EconfStatus::StatusOk
}

#[unsafe(no_mangle)]
pub extern "C" fn econf_save(interface: *const CInterfaceInstance) -> EconfStatus {
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let mut interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
        match interface.save() {
            Ok(_) => {
                EconfStatus::StatusOk
            }
            Err(_) => EconfStatus::StatusError,
        }
    });
    EconfStatus::StatusOk
}
