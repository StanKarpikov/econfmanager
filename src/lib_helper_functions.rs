use std::{any::type_name, time::Duration};

use log::{debug, error};

use crate::{interface::generated::ParameterId, schema::ParameterType, CInterfaceInstance, EconfStatus};

const LOCK_TRYING_DURATION: Duration = Duration::from_secs(1);

pub(crate) fn get_parameter<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    let interface = unsafe { &*interface };
    debug!("Get ID {}:{}", id as usize, type_name::<T>());
    interface.with_lock(|lock| {
        let interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
        match interface.get(id, false) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    EconfStatus::StatusOk
                } else {
                    error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                    EconfStatus::StatusError
                }
            }
            Err(e) => {
                error!("Error getting ID {}:{} - {}", id as usize, type_name::<T>(), e);
                EconfStatus::StatusError
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
    let interface = unsafe { &*interface };
    interface.with_lock(|lock| {
        let interface = match lock.try_lock_for(LOCK_TRYING_DURATION) {
            Some(guard) => guard,
            None => {
                error!("Failed to acquire lock within timeout");
                return EconfStatus::StatusError;
            }
        };
        let parameter = unsafe { (*out_parameter).clone() };
        match interface.set(id, parameter.to_parameter_value()) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    EconfStatus::StatusOk
                } else {
                    error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                    EconfStatus::StatusError
                }
            }
            Err(e) => {
                error!("Error setting ID {}:{} - {}", id as usize, type_name::<T>(), e);
                EconfStatus::StatusError
            }
        }
    })
}