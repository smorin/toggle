//! C ABI for togl (`libtogl`). All exported symbols are `togl_`-prefixed,
//! return an `int` status, and never unwind across the FFI boundary.

// Exported functions dereference caller-provided pointers by C-API contract;
// their safety is documented in the header, not via Rust `unsafe` signatures.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod error;
mod introspect;
mod mem;
mod transforms;

pub use introspect::*;
pub use transforms::*;

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

    #[test]
    fn toggle_comments_roundtrip() {
        let content = CString::new("a\nb\nc\n").unwrap();
        let ranges = [ToglRange { start: 1, end: 2 }];
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_toggle_comments(content.as_ptr(), ranges.as_ptr(), ranges.len(), 1, &mut out);
        assert_eq!(rc, 0);
        let s = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        assert!(s.contains("# a"), "got: {s}");
        togl_string_free(out);
    }

    #[test]
    fn toggle_comments_null_content_errors() {
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_toggle_comments(std::ptr::null(), std::ptr::null(), 0, 0, &mut out);
        assert_eq!(rc, -1);
    }

    #[test]
    fn toggle_comments_bad_force_errors() {
        let content = CString::new("a\n").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_toggle_comments(content.as_ptr(), std::ptr::null(), 0, 99, &mut out);
        assert_eq!(rc, -5);
    }

    const VARIANTS: &str = "# toggle:start ID=db:sqlite\nimport sqlite3\n# toggle:end ID=db:sqlite\n# toggle:start ID=db:postgres\n# import psycopg2\n# toggle:end ID=db:postgres\n";

    #[test]
    fn activate_variant_default_marker() {
        let content = CString::new(VARIANTS).unwrap();
        let group = CString::new("db").unwrap();
        let variant = CString::new("postgres").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_activate_variant(
            content.as_ptr(),
            group.as_ptr(),
            variant.as_ptr(),
            std::ptr::null(),
            &mut out,
        );
        assert_eq!(rc, 0);
        togl_string_free(out);
    }

    #[test]
    fn find_and_toggle_section_default_marker() {
        let content = CString::new("# toggle:start ID=foo\nx\n# toggle:end ID=foo\n").unwrap();
        let id = CString::new("foo").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc =
            togl_find_and_toggle_section(content.as_ptr(), id.as_ptr(), std::ptr::null(), &mut out);
        assert_eq!(rc, 0);
        togl_string_free(out);
    }

    #[test]
    fn discover_sections_returns_json_array() {
        let content = CString::new("# toggle:start ID=foo\nx\n# toggle:end ID=foo\n").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_discover_sections(content.as_ptr(), &mut out);
        assert_eq!(rc, 0);
        let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 1);
        togl_string_free(out);
    }

    #[test]
    fn scan_sections_json_has_file_label() {
        let path = CString::new("memory.py").unwrap();
        let content = CString::new("# toggle:start ID=foo\nx\n# toggle:end ID=foo\n").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_scan_sections(path.as_ptr(), content.as_ptr(), &mut out);
        assert_eq!(rc, 0);
        let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        assert!(json.contains("memory.py"), "got: {json}");
        togl_string_free(out);
    }

    #[test]
    fn validate_sections_returns_json_array() {
        // Unclosed section → at least one issue.
        let content = CString::new("# toggle:start ID=foo\nx\n").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let rc = togl_validate_sections(content.as_ptr(), 0, &mut out);
        assert_eq!(rc, 0);
        let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert!(v.is_array());
        togl_string_free(out);
    }
}
