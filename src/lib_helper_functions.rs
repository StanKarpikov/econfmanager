use crate::{interface::generated::ParameterId, schema::ParameterValue, CInterfaceInstance, EconfStatus};


pub(crate) fn get_i32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut i32) -> EconfStatus {
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

pub(crate) fn set_i32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut i32) -> EconfStatus {
    let instance = instance.as_ref();
    let parameter = unsafe { (*out_parameter).clone()};
    match instance.set(id, ParameterValue::ValI32(parameter)) {
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

pub(crate) fn get_f32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut f32) -> EconfStatus {
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

pub(crate) fn set_f32(instance: CInterfaceInstance, id: ParameterId, out_parameter: *mut f32) -> EconfStatus {
    let instance = instance.as_ref();
    let parameter = unsafe { (*out_parameter).clone()};
    match instance.set(id, ParameterValue::ValF32(parameter)) {
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