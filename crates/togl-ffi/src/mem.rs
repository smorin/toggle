//! Boundary helpers: borrow C strings, return owned C strings, catch panics.

use crate::error::{TOGL_ERR_INVALID_UTF8, TOGL_ERR_NULL_POINTER, TOGL_ERR_PANIC};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Borrow a required `*const c_char` as `&str`. Returns Err(code) on null/invalid UTF-8.
///
/// # Safety
/// `p`, if non-null, must point to a valid NUL-terminated C string.
pub fn borrow_str<'a>(p: *const c_char) -> Result<&'a str, i32> {
    if p.is_null() {
        return Err(TOGL_ERR_NULL_POINTER);
    }
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|_| TOGL_ERR_INVALID_UTF8)
}

/// Borrow an optional `*const c_char`, returning `default` when null.
///
/// # Safety
/// `p`, if non-null, must point to a valid NUL-terminated C string.
pub fn borrow_str_or(p: *const c_char, default: &str) -> Result<&str, i32> {
    if p.is_null() {
        Ok(default)
    } else {
        borrow_str(p)
    }
}

/// Move an owned String across the boundary as a heap `char*` (caller frees via togl_string_free).
/// Returns Err(TOGL_ERR_INVALID_ARGUMENT) if the string contains an interior NUL.
pub fn out_string(s: String, out: *mut *mut c_char) -> Result<(), i32> {
    if out.is_null() {
        return Err(TOGL_ERR_NULL_POINTER);
    }
    let c = CString::new(s).map_err(|_| crate::error::TOGL_ERR_INVALID_ARGUMENT)?;
    unsafe { *out = c.into_raw() };
    Ok(())
}

/// Run a fallible boundary closure, converting panics into TOGL_ERR_PANIC.
pub fn guard(f: impl FnOnce() -> Result<(), i32>) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => crate::error::TOGL_OK,
        Ok(Err(code)) => code,
        Err(_) => TOGL_ERR_PANIC,
    }
}
