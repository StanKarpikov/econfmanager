use std::{any::type_name, time::Duration};

use log::{debug, error};

use crate::{generated::ParameterId, schema::ParameterType, CInterfaceInstance, EconfStatus, InterfaceInstance};

const LOCK_TRYING_DURATION: Duration = Duration::from_secs(1);

macro_rules! validate_ptr {
    ($ptr:expr, $type:ty) => {
        if $ptr.is_null() {
            error!("Null pointer provided to {}", stringify!($ptr));
            return EconfStatus::StatusError;
        }
        // if ($ptr as usize) % std::mem::align_of::<$type>() != 0 {
        //     error!("Unaligned pointer provided to {}", stringify!($ptr));
        //     return EconfStatus::StatusError;
        // }
    };
}

pub(crate) fn interface_execute<F>(
    interface: *const CInterfaceInstance, 
    f: F
) -> EconfStatus 
where
    F: FnOnce(&mut InterfaceInstance) -> Result<(), Box<dyn std::error::Error>>,
{
    validate_ptr!(interface, CInterfaceInstance);
    
    let interface = unsafe { &*interface };
    match interface.with_lock(|lock| {
        lock.try_lock_for(LOCK_TRYING_DURATION)
            .map(|mut guard| f(&mut *guard))
            .unwrap_or_else(|| {
                error!("Failed to acquire lock within timeout");
                Err("Lock timeout".into())
            })
            .map(|_| EconfStatus::StatusOk)
            .unwrap_or_else(|e| {
                error!("Operation failed: {}", e);
                EconfStatus::StatusError
            })
    }){
        Ok(status) => status,
        Err(_) => EconfStatus::StatusError,
    }
}

pub(crate) fn get_parameter<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    debug!("Get ID {}:{}", id as usize, type_name::<T>());
    interface_execute(interface, |interface| {
        match interface.get(id, false) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    Ok(())
                } else {
                    error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                    Err(format!("Error converting ID {}:{}", id as usize, type_name::<T>()).into())
                }
            }
            Err(e) => {
                error!("Error getting ID {}:{} - {}", id as usize, type_name::<T>(), e);
                Err(format!("Error getting ID {}:{} - {}", id as usize, type_name::<T>(), e).into())
            }
        }
    })
}

pub(crate) fn set_parameter<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    debug!("Set ID {}:{}", id as usize, type_name::<T>());
    interface_execute(interface, |interface| {
        let parameter = unsafe { (*out_parameter).clone() };
        match interface.set(id, parameter.to_parameter_value()) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    Ok(())
                } else {
                    error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                    Err(format!("Error converting ID {}:{}", id as usize, type_name::<T>()).into())
                }
            }
            Err(e) => {
                error!("Error setting ID {}:{} - {}", id as usize, type_name::<T>(), e);
                Err(format!("Error setting ID {}:{} - {}", id as usize, type_name::<T>(), e).into())
            }
        }
    })
}