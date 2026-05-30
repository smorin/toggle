//! C ABI for togl (`libtogl`). All exported symbols are `togl_`-prefixed,
//! return an `int` status, and never unwind across the FFI boundary.

// Exported functions dereference caller-provided pointers by C-API contract;
// their safety is documented in the header, not via Rust `unsafe` signatures.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod error;
mod mem;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::OnceLock;

/// ABI version for runtime negotiation. Bump on any breaking C-ABI change.
pub const TOGL_ABI_VERSION: u32 = 1;

/// Crate version as a static NUL-terminated string (never freed by the caller).
#[no_mangle]
pub extern "C" fn togl_version() -> *const c_char {
    static V: OnceLock<CString> = OnceLock::new();
    V.get_or_init(|| CString::new(env!("CARGO_PKG_VERSION")).unwrap())
        .as_ptr()
}

/// Integer ABI version for runtime negotiation.
#[no_mangle]
pub extern "C" fn togl_abi_version() -> u32 {
    TOGL_ABI_VERSION
}

/// Human-readable message for a status code (static; do not free).
#[no_mangle]
pub extern "C" fn togl_error_message(code: c_int) -> *const c_char {
    use std::collections::HashMap;
    static MSGS: OnceLock<HashMap<i32, CString>> = OnceLock::new();
    let map = MSGS.get_or_init(|| {
        [0, -1, -2, -3, -4, -5, i32::MIN]
            .into_iter()
            .map(|c| (c, CString::new(error::message(c)).unwrap()))
            .collect()
    });
    map.get(&code)
        .unwrap_or_else(|| map.get(&i32::MIN).unwrap())
        .as_ptr()
}

/// Free a `char*` previously returned by this library via an out-pointer.
/// Safe to call with NULL. Never call C `free()` on togl strings.
#[no_mangle]
pub extern "C" fn togl_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn version_is_nonempty_static() {
        let p = togl_version();
        assert!(!p.is_null());
        let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap();
        assert!(!s.is_empty());
    }

    #[test]
    fn abi_version_matches_constant() {
        assert_eq!(togl_abi_version(), TOGL_ABI_VERSION);
    }

    #[test]
    fn error_message_known_and_unknown() {
        let ok = unsafe { CStr::from_ptr(togl_error_message(0)) }
            .to_str()
            .unwrap();
        assert_eq!(ok, "ok");
        let unk = unsafe { CStr::from_ptr(togl_error_message(-999)) }
            .to_str()
            .unwrap();
        assert_eq!(unk, "unknown error");
    }

    #[test]
    fn string_free_accepts_null_and_owned() {
        togl_string_free(std::ptr::null_mut()); // no-op, no crash
        let owned = CString::new("hi").unwrap().into_raw();
        togl_string_free(owned); // frees without double-free
    }
}
