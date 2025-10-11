use std::{
    any::type_name,
    ffi::{CStr, CString, c_char},
    ptr, slice,
    time::Duration,
};

use log::{debug, error};

use crate::{
    CInterfaceInstance, EconfStatus, InterfaceInstance,
    generated::ParameterId,
    schema::{ParameterType, ParameterValue},
};

const LOCK_TRYING_DURATION: Duration = Duration::from_secs(1);

macro_rules! validate_ptr {
    ($ptr:expr, $type:ty) => {
        if $ptr.is_null() {
            error!("Null pointer provided to {}", stringify!($ptr));
            return EconfStatus::StatusError;
        }
    };
}

pub(crate) fn interface_execute<F>(interface: *const CInterfaceInstance, f: F) -> EconfStatus
where
    F: FnOnce(&mut InterfaceInstance) -> Result<(), Box<dyn std::error::Error>>,
{
    validate_ptr!(interface, CInterfaceInstance);

    let interface = unsafe { &*interface };
    match interface.with_lock(|lock| {
        lock.try_lock_for(LOCK_TRYING_DURATION)
            .map(|mut guard| f(&mut guard))
            .unwrap_or_else(|| {
                error!("Failed to acquire lock within timeout");
                Err("Lock timeout".into())
            })
            .map(|_| EconfStatus::StatusOk)
            .unwrap_or_else(|e| {
                error!("Operation failed: {}", e);
                EconfStatus::StatusError
            })
    }) {
        Ok(status) => status,
        Err(_) => EconfStatus::StatusError,
    }
}

pub(crate) fn get_parameter_quick<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
) -> T {
    debug!("Get ID {} quick:{}", id as usize, type_name::<T>());
    let mut out_parameter = None;
    interface_execute(interface, |interface| match interface.get(id, false) {
        Ok(parameter) => {
            if let Some(ret_val) = T::from_parameter_value(parameter.clone()) {
                out_parameter = Some(ret_val);
                Ok(())
            } else if let ParameterValue::ValEnum(val) = parameter {
                if let Some(ret_val) = T::from_parameter_value(ParameterValue::ValI32(val))
                {
                    out_parameter = Some(ret_val);
                    Ok(())
                }
                else {
                    error!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>());
                    Err(format!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>()).into())
                }
            } else {
                error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                Err(format!("Error converting ID {}:{}", id as usize, type_name::<T>()).into())
            }
        }
        Err(e) => {
            error!(
                "Error getting ID {}:{} - {}",
                id as usize,
                type_name::<T>(),
                e
            );
            Err(format!(
                "Error getting ID {}:{} - {}",
                id as usize,
                type_name::<T>(),
                e
            )
            .into())
        }
    });
    match out_parameter {
        Some(val) => val,
        // TODO: This may not be correct
        None => T::from_parameter_value(ParameterValue::ValNone).unwrap(),
    }
}

pub(crate) fn get_parameter<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_parameter: *mut T,
) -> EconfStatus {
    debug!("Get ID {}:{}", id as usize, type_name::<T>());
    interface_execute(interface, |interface| match interface.get(id, false) {
        Ok(parameter) => {
            if let Some(ret_val) = T::from_parameter_value(parameter.clone()) {
                if out_parameter.is_null() {
                    error!("Null pointer provided for {}", id as usize);
                    return Err(format!("Null pointer provided for {}", id as usize).into());
                }
                unsafe { *out_parameter = ret_val };
                Ok(())
            } else if let ParameterValue::ValEnum(val) = parameter {
                if let Some(ret_val) = T::from_parameter_value(ParameterValue::ValI32(val))
                {
                    if out_parameter.is_null() {
                        error!("Null pointer provided for {}", id as usize);
                        return Err(format!("Null pointer provided for {}", id as usize).into());
                    }
                    unsafe { *out_parameter = ret_val };
                    Ok(())
                }
                else {
                    error!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>());
                    Err(format!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>()).into())
                }
            }else {
                error!("Error converting ID {}:{} paraemeter {}", id as usize, type_name::<T>(), &parameter);
                Err(format!("Error converting ID {}:{} paraemeter {}", id as usize, type_name::<T>(), &parameter).into())
            }
        }
        Err(e) => {
            error!(
                "Error getting ID {}:{} - {}",
                id as usize,
                type_name::<T>(),
                e
            );
            Err(format!(
                "Error getting ID {}:{} - {}",
                id as usize,
                type_name::<T>(),
                e
            )
            .into())
        }
    })
}

pub(crate) fn set_parameter<T: ParameterType>(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    parameter: T,
    out_parameter: *mut T,
) -> EconfStatus {
    debug!("Set ID {}:{}", id as usize, type_name::<T>());
    interface_execute(interface, |interface| {
        match interface.set(id, parameter.to_parameter_value()) {
            Ok(parameter) => {
                if let Some(ret_val) = T::from_parameter_value(parameter.clone()) {
                    if !out_parameter.is_null() {
                        unsafe { *out_parameter = ret_val };
                    }
                    Ok(())
                } else if let ParameterValue::ValEnum(val) = parameter {
                    if let Some(ret_val) = T::from_parameter_value(ParameterValue::ValI32(val))
                    {
                        if !out_parameter.is_null() {
                            unsafe { *out_parameter = ret_val };
                        }
                        Ok(())
                    }
                    else {
                        error!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>());
                        Err(format!("Error converting ID for Enum {}:{}", id as usize, type_name::<T>()).into())
                    }
                } else {
                    error!("Error converting ID {}:{}", id as usize, type_name::<T>());
                    Err(format!("Error converting ID {}:{}", id as usize, type_name::<T>()).into())
                }
            }
            Err(e) => {
                error!(
                    "Error setting ID {}:{} - {}",
                    id as usize,
                    type_name::<T>(),
                    e
                );
                Err(format!(
                    "Error setting ID {}:{} - {}",
                    id as usize,
                    type_name::<T>(),
                    e
                )
                .into())
            }
        }
    })
}

pub unsafe fn copy_string_to_c_buffer(
    s: &str,
    out_c_string: *mut c_char,
    max_len: usize,
    id: ParameterId,
) -> Result<usize, String> {
    let c_str = match CString::new(s) {
        Ok(c) => c,
        Err(e) => {
            let err = format!("String contains null bytes: {} for {}", e, id as usize);
            return Err(err);
        }
    };

    let bytes = c_str.as_bytes_with_nul();
    
    if out_c_string.is_null() {
        return Ok(bytes.len());
    }

    if bytes.len() > max_len {
        return Ok(bytes.len());
    }

    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, out_c_string, bytes.len()) };

    Ok(bytes.len())
}

fn c_char_to_string(c_string: *const c_char, id: ParameterId) -> Result<String, String> {
    if c_string.is_null() {
        return Err(format!("Null pointer provided for {}", id as usize));
    }

    unsafe {
        CStr::from_ptr(c_string)
            .to_str()
            .map(|s| s.to_owned())
            .map_err(|e| format!("Invalid UTF-8 string: {} for {}", e, id as usize))
    }
}

pub(crate) fn get_string(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_c_string: *mut c_char,
    max_len: usize,
    out_len: *mut usize,
) -> EconfStatus {
    debug!("Get ID {}: string", id as usize);
    interface_execute(interface, |interface| match interface.get(id, false) {
        Ok(parameter) => match parameter {
            ParameterValue::ValString(val_str) => {
                let bytes_copied = unsafe { copy_string_to_c_buffer(&val_str, out_c_string, max_len, id)? };
                if !out_len.is_null(){
                    unsafe { *out_len = bytes_copied };
                }
                Ok(())
            }
            _ => {
                Err(format!("Wrong type requested for ID {}: string", id as usize).into())
            }
        },
        Err(e) => Err(format!("Error getting ID {}: string - {}", id as usize, e).into()),
    })
}

pub(crate) fn set_string(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    c_string: *const c_char,
) -> EconfStatus {
    debug!("Set ID {}: string", id as usize);
    interface_execute(interface, |interface| {
        let rust_string = match c_char_to_string(c_string, id) {
            Ok(s) => s,
            Err(e) => {
                error!("Invalid string for ID {}: {}", id as usize, e);
                return Err(e.into());
            }
        };
        let parameter = ParameterValue::ValString(rust_string.into());
        match interface.set(id, parameter) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error setting ID {}: string - {}", id as usize, e).into()),
        }
    })
}

pub unsafe fn copy_blob_to_c_buffer(
    blob: &[u8],
    out_buffer: *mut u8,
    max_len: usize
) -> Result<usize, String> {
    if out_buffer.is_null() {
        /* Return the length if the buffer was NULL */
        return Ok(blob.len());
    }

    if blob.len() > max_len {
        /* Return the length if the buffer doesn't fit */
        return Ok(blob.len());
    }

    unsafe { ptr::copy_nonoverlapping(blob.as_ptr(), out_buffer, blob.len()) };
    Ok(blob.len())
}

/// Converts a C-style buffer to a Rust Vec<u8>
///
/// # Safety
/// 
/// This function is unsafe because it dereferences raw pointers. The caller must ensure:
/// - `buffer` points to valid memory and is properly aligned for `u8`
/// - `buffer` points to at least `len` bytes of initialized memory
/// - The memory region `[buffer, buffer + len)` must be valid for the duration of the function call
/// - The memory must not be modified by other threads during this operation
pub unsafe fn c_buffer_to_blob(buffer: *const u8, len: usize, id: ParameterId) -> Result<Vec<u8>, String> {
    if buffer.is_null() {
        return Err(format!("Null pointer provided for blob ID {}", id as usize));
    }

    Ok(unsafe { slice::from_raw_parts(buffer, len).to_vec() })
}

#[allow(dead_code)]
pub(crate) fn get_blob(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    out_buffer: *mut u8,
    max_len: usize,
    out_len: *mut usize,
) -> EconfStatus {
    debug!("Get ID {}: blob", id as usize);
    interface_execute(interface, |interface| match interface.get(id, false) {
        Ok(parameter) => match parameter {
            ParameterValue::ValBlob(blob) => {
                let bytes_copied = unsafe { copy_blob_to_c_buffer(&blob, out_buffer, max_len)? };
                if !out_len.is_null(){
                    unsafe { *out_len = bytes_copied };
                }
                Ok(())
            }
            _ => Err(format!("Wrong type requested for ID {}: blob", id as usize).into()),
        },
        Err(e) => Err(format!("Error getting ID {}: blob - {}", id as usize, e).into()),
    })
}

#[allow(dead_code)]
pub(crate) fn set_blob(
    interface: *const CInterfaceInstance,
    id: ParameterId,
    buffer: *const u8,
    len: usize,
) -> EconfStatus {
    debug!("Set ID {}: blob ({} bytes)", id as usize, len);
    interface_execute(interface, |interface| {
        let blob = unsafe { c_buffer_to_blob(buffer, len, id)? };
        let parameter = ParameterValue::ValBlob(blob);
        match interface.set(id, parameter) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error setting ID {}: blob - {}", id as usize, e).into()),
        }
    })
}
