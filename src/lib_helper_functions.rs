use crate::{interface::generated::ParameterId, schema::ParameterType, CInterfaceInstance, EconfStatus};

pub(crate) fn get_parameter<T: ParameterType>(
    interface: CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    interface.with_lock(|lock| {
        let interface = lock.lock().unwrap();
        match interface.get(id, false) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    EconfStatus::StatusOk
                } else {
                    EconfStatus::StatusError
                }
            }
            Err(_) => EconfStatus::StatusError,
        }
    })
}

pub(crate) fn set_parameter<T: ParameterType>(
    interface: CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    interface.with_lock(|lock| {
        let interface = lock.lock().unwrap();
        let parameter = unsafe { (*out_parameter).clone() };
        match interface.set(id, parameter.to_parameter_value()) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter) {
                    unsafe { *out_parameter = ret_val };
                    EconfStatus::StatusOk
                } else {
                    EconfStatus::StatusError
                }
            }
            Err(_) => EconfStatus::StatusError,
        }
    })
}