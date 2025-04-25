use crate::{interface::generated::ParameterId, schema::ParameterType, CInterfaceInstance, EconfStatus};

pub(crate) fn get_parameter<T: ParameterType>(
    interface: CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    let interface = interface.as_ref();
    match interface.get(id) {
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
}

pub(crate) fn set_parameter<T: ParameterType>(
    interface: CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    let interface = interface.as_ref();
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
}