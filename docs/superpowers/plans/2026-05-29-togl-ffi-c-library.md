# togl-ffi C Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add `crates/togl-ffi` to the existing workspace, producing an ABI-stable C library `libtogl` (static `.a` + shared `.so`/`.dylib`) plus a committed C header and pkg-config file, wrapping the string-core + introspection subset of `togl-lib`.

**Architecture:** New workspace member `togl-ffi` (package `togl-ffi`, `[lib] name = "togl"`, `crate-type=["staticlib","cdylib"]`) depends on `togl-lib` and exposes a flat `extern "C"` surface. All complex results cross the boundary as JSON strings; only `ToglRange` is a boundary struct. Every function returns an `int` status, delivers output via out-pointers, and is wrapped in `catch_unwind`. cbindgen generates `include/togl.h` (committed). A C smoke test links the library and exercises it.

**Tech Stack:** Rust (edition 2021), cbindgen, serde_json, libc-free C ABI, cc (for the smoke test).

**Source spec:** `docs/superpowers/specs/2026-05-29-togl-c-abi-library-design.md`.

## Resolved C API (final)

```c
typedef struct { size_t start; size_t end; } ToglRange;

/* status: 0 = TOGL_OK; negative = error (see togl_error_message) */
const char *togl_version(void);            /* static; do NOT free */
uint32_t    togl_abi_version(void);
const char *togl_error_message(int code);  /* static; do NOT free */
void        togl_string_free(char *s);     /* free any char* the lib returned via out-ptr */

/* transforms: out_result = new content (caller frees via togl_string_free) */
int togl_toggle_comments(const char *content, const ToglRange *ranges,
                         size_t range_count, int force_mode, char **out_result);
int togl_find_and_toggle_section(const char *content, const char *section_id,
                                 const char *comment_marker, char **out_result);
int togl_activate_variant(const char *content, const char *group, const char *variant,
                          const char *comment_marker, char **out_result);

/* introspection: out_json = JSON string (caller frees via togl_string_free) */
int togl_discover_sections(const char *content, char **out_json);
int togl_scan_sections(const char *path, const char *content, char **out_json);
int togl_validate_sections(const char *content, int pair_only, char **out_json);
```

- `force_mode`: `0` = invert (None), `1` = on (`Some("on")`), `2` = off (`Some("off")`); any other → `TOGL_ERR_INVALID_ARGUMENT`.
- `comment_marker`: if NULL, defaults to `"#"`.
- `pair_only`: `0`/non-zero → `bool`.
- Error codes: `0` OK, `-1` NULL_POINTER, `-2` INVALID_UTF8, `-3` PANIC, `-4` OPERATION (lib `Err`), `-5` INVALID_ARGUMENT.

---

### Task 1: Scaffold the `togl-ffi` crate

**Files:**
- Create: `crates/togl-ffi/Cargo.toml`, `crates/togl-ffi/src/lib.rs`
- Modify: `Cargo.toml` (root members)

- [ ] **Step 1: Add the crate to the workspace members**

In root `Cargo.toml`, change:
```toml
members = ["crates/togl-lib", "crates/togl-cli"]
```
to:
```toml
members = ["crates/togl-lib", "crates/togl-cli", "crates/togl-ffi"]
```

- [ ] **Step 2: Create `crates/togl-ffi/Cargo.toml`**

```toml
[package]
name = "togl-ffi"
description = "C ABI (libtogl) for togl — toggling code comments across languages"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[lib]
name = "togl"
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
togl-lib = { path = "../togl-lib" }
serde.workspace = true
serde_json = "1"

[build-dependencies]
cbindgen = "0.27"
```
(`rlib` is included so the crate's own `#[test]` unit tests can link it; `staticlib`+`cdylib` produce `libtogl.a` and `libtogl.so`/`.dylib`.)

- [ ] **Step 3: Create a minimal `crates/togl-ffi/src/lib.rs`**

```rust
//! C ABI for togl (`libtogl`). All exported symbols are `togl_`-prefixed,
//! return an `int` status, and never unwind across the FFI boundary.

/// ABI version for runtime negotiation. Bump on any breaking C-ABI change.
pub const TOGL_ABI_VERSION: u32 = 1;
```

- [ ] **Step 4: Verify it builds and emits both library kinds**

Run: `cargo build -p togl-ffi`
Expected: PASS.

Run: `ls target/debug | grep -E 'libtogl\.(a|so|dylib)'`
Expected: shows `libtogl.a` and `libtogl.so` (Linux) or `libtogl.dylib` (macOS).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/togl-ffi/
git commit -m "feat(ffi): scaffold togl-ffi crate producing libtogl (static+shared)"
```

---

### Task 2: Error codes, panic guard, and string/metadata lifecycle

**Files:**
- Create: `crates/togl-ffi/src/error.rs`, `crates/togl-ffi/src/mem.rs`
- Modify: `crates/togl-ffi/src/lib.rs`

- [ ] **Step 1: Write failing tests** in `crates/togl-ffi/src/lib.rs` (append a `#[cfg(test)] mod tests`):

```rust
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
        let ok = unsafe { CStr::from_ptr(togl_error_message(0)) }.to_str().unwrap();
        assert_eq!(ok, "ok");
        let unk = unsafe { CStr::from_ptr(togl_error_message(-999)) }.to_str().unwrap();
        assert_eq!(unk, "unknown error");
    }

    #[test]
    fn string_free_accepts_null_and_owned() {
        togl_string_free(std::ptr::null_mut()); // no-op, no crash
        let owned = CString::new("hi").unwrap().into_raw();
        togl_string_free(owned); // frees without double-free
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p togl-ffi`
Expected: FAIL (functions not defined).

- [ ] **Step 3: Create `crates/togl-ffi/src/error.rs`**

```rust
//! Stable C error codes. Values are part of the ABI and never change meaning.

pub const TOGL_OK: i32 = 0;
pub const TOGL_ERR_NULL_POINTER: i32 = -1;
pub const TOGL_ERR_INVALID_UTF8: i32 = -2;
pub const TOGL_ERR_PANIC: i32 = -3;
pub const TOGL_ERR_OPERATION: i32 = -4;
pub const TOGL_ERR_INVALID_ARGUMENT: i32 = -5;

pub fn message(code: i32) -> &'static str {
    match code {
        TOGL_OK => "ok",
        TOGL_ERR_NULL_POINTER => "null pointer argument",
        TOGL_ERR_INVALID_UTF8 => "input was not valid UTF-8",
        TOGL_ERR_PANIC => "internal panic",
        TOGL_ERR_OPERATION => "operation failed",
        TOGL_ERR_INVALID_ARGUMENT => "invalid argument",
        _ => "unknown error",
    }
}
```

- [ ] **Step 4: Create `crates/togl-ffi/src/mem.rs`**

```rust
//! Boundary helpers: borrow C strings, return owned C strings, catch panics.

use crate::error::{TOGL_ERR_INVALID_UTF8, TOGL_ERR_NULL_POINTER, TOGL_ERR_PANIC};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Borrow a required `*const c_char` as `&str`. Returns Err(code) on null/invalid UTF-8.
pub fn borrow_str<'a>(p: *const c_char) -> Result<&'a str, i32> {
    if p.is_null() {
        return Err(TOGL_ERR_NULL_POINTER);
    }
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|_| TOGL_ERR_INVALID_UTF8)
}

/// Borrow an optional `*const c_char`, returning `default` when null.
pub fn borrow_str_or<'a>(p: *const c_char, default: &'a str) -> Result<&'a str, i32> {
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
```

- [ ] **Step 5: Replace `crates/togl-ffi/src/lib.rs` body** (keep the test module) with:

```rust
//! C ABI for togl (`libtogl`). All exported symbols are `togl_`-prefixed,
//! return an `int` status, and never unwind across the FFI boundary.

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
```

(Note: `togl_error_message` maps `i32::MIN` to "unknown error" so any unmapped code returns a valid static pointer.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p togl-ffi`
Expected: PASS (4 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/togl-ffi/
git commit -m "feat(ffi): error codes, panic guard, string + metadata lifecycle"
```

---

### Task 3: Transform functions

**Files:**
- Create: `crates/togl-ffi/src/transforms.rs`
- Modify: `crates/togl-ffi/src/lib.rs` (add `mod transforms;`)

- [ ] **Step 1: Write failing tests** — append to the `tests` module in `lib.rs`:

```rust
#[test]
fn toggle_comments_roundtrip() {
    use std::ffi::CString;
    let content = CString::new("a\nb\nc\n").unwrap();
    let ranges = [ToglRange { start: 1, end: 2 }];
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_toggle_comments(content.as_ptr(), ranges.as_ptr(), ranges.len(), 1, &mut out);
    assert_eq!(rc, 0);
    let s = unsafe { std::ffi::CStr::from_ptr(out) }.to_str().unwrap().to_owned();
    assert!(s.contains("# a"));
    togl_string_free(out);
}

#[test]
fn toggle_comments_null_content_errors() {
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_toggle_comments(std::ptr::null(), std::ptr::null(), 0, 0, &mut out);
    assert_eq!(rc, -1);
}

#[test]
fn toggle_comments_bad_force_errors() {
    use std::ffi::CString;
    let content = CString::new("a\n").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_toggle_comments(content.as_ptr(), std::ptr::null(), 0, 99, &mut out);
    assert_eq!(rc, -5);
}

#[test]
fn activate_variant_default_marker() {
    use std::ffi::CString;
    // group "db" with two variants, postgres active-by-default style
    let content = CString::new(
        "# ID=db:postgres\nprint(1)\n# ID=db:mysql\n# print(2)\n"
    ).unwrap();
    let group = CString::new("db").unwrap();
    let variant = CString::new("mysql").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_activate_variant(content.as_ptr(), group.as_ptr(), variant.as_ptr(),
                                   std::ptr::null(), &mut out);
    assert_eq!(rc, 0);
    togl_string_free(out);
}
```

Also add the `ToglRange` type — it must be defined before tests use it. Put it in `transforms.rs` and re-export from `lib.rs` (Step 3/4).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p togl-ffi`
Expected: FAIL (functions/`ToglRange` not defined).

- [ ] **Step 3: Create `crates/togl-ffi/src/transforms.rs`**

```rust
//! Transform functions: content in → new content out.

use crate::error::{TOGL_ERR_INVALID_ARGUMENT, TOGL_ERR_OPERATION};
use crate::mem::{borrow_str, borrow_str_or, guard, out_string};
use std::os::raw::{c_char, c_int};
use togl_lib::core::{
    activate_variant, find_and_toggle_section, toggle_comments, CommentStyle, LineRange,
};

/// A line range `[start, end]`, 1-based inclusive. Frozen POD — never change layout.
#[repr(C)]
pub struct ToglRange {
    pub start: usize,
    pub end: usize,
}

fn force_str(force_mode: c_int) -> Result<Option<&'static str>, i32> {
    match force_mode {
        0 => Ok(None),
        1 => Ok(Some("on")),
        2 => Ok(Some("off")),
        _ => Err(TOGL_ERR_INVALID_ARGUMENT),
    }
}

fn comment_style(marker: &str) -> CommentStyle {
    CommentStyle {
        single_line: marker.to_string(),
        multi_line_start: None,
        multi_line_end: None,
    }
}

#[no_mangle]
pub extern "C" fn togl_toggle_comments(
    content: *const c_char,
    ranges: *const ToglRange,
    range_count: usize,
    force_mode: c_int,
    out_result: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let force = force_str(force_mode)?;
        let ranges: Vec<LineRange> = if ranges.is_null() || range_count == 0 {
            Vec::new()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(ranges, range_count) };
            slice.iter().map(|r| LineRange::new(r.start, r.end)).collect()
        };
        let result = toggle_comments(content, &ranges, force);
        out_string(result, out_result)
    })
}

#[no_mangle]
pub extern "C" fn togl_find_and_toggle_section(
    content: *const c_char,
    section_id: *const c_char,
    comment_marker: *const c_char,
    out_result: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let section_id = borrow_str(section_id)?;
        let marker = borrow_str_or(comment_marker, "#")?;
        let style = comment_style(marker);
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        find_and_toggle_section(&mut lines, section_id, &None, &style)
            .map_err(|_| TOGL_ERR_OPERATION)?;
        out_string(lines.join("\n"), out_result)
    })
}

#[no_mangle]
pub extern "C" fn togl_activate_variant(
    content: *const c_char,
    group: *const c_char,
    variant: *const c_char,
    comment_marker: *const c_char,
    out_result: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let group = borrow_str(group)?;
        let variant = borrow_str(variant)?;
        let marker = borrow_str_or(comment_marker, "#")?;
        let style = comment_style(marker);
        let result =
            activate_variant(content, group, variant, &style).map_err(|_| TOGL_ERR_OPERATION)?;
        out_string(result, out_result)
    })
}
```

- [ ] **Step 4: Add to `lib.rs`** — insert `mod transforms;` and `pub use transforms::ToglRange;` after `mod mem;`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p togl-ffi`
Expected: PASS (8 tests). If `activate_variant_default_marker` fails on the fixture, adjust the fixture content in the test to a valid two-variant group per `togl-lib` marker syntax (consult `crates/togl-lib/tests/unit/core_tests.rs` for a known-good `VARIANTS_FIXTURE`).

- [ ] **Step 6: Commit**

```bash
git add crates/togl-ffi/
git commit -m "feat(ffi): transform functions (toggle_comments, section toggle, activate_variant)"
```

---

### Task 4: Introspection functions (JSON results)

**Files:**
- Create: `crates/togl-ffi/src/introspect.rs`
- Modify: `crates/togl-ffi/src/lib.rs` (add `mod introspect;`)

- [ ] **Step 1: Write failing tests** — append to the `tests` module:

```rust
#[test]
fn discover_sections_returns_json_array() {
    use std::ffi::{CStr, CString};
    let content = CString::new("# ID=foo\nx\n# ID=foo end\n").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_discover_sections(content.as_ptr(), &mut out);
    assert_eq!(rc, 0);
    let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(v.is_array());
    togl_string_free(out);
}

#[test]
fn scan_sections_json_has_file_label() {
    use std::ffi::{CStr, CString};
    let path = CString::new("memory.py").unwrap();
    let content = CString::new("# ID=foo\nx\n# ID=foo end\n").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_scan_sections(path.as_ptr(), content.as_ptr(), &mut out);
    assert_eq!(rc, 0);
    let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
    assert!(json.contains("memory.py"));
    togl_string_free(out);
}

#[test]
fn validate_sections_returns_json() {
    use std::ffi::{CStr, CString};
    let content = CString::new("# ID=foo\nx\n").unwrap(); // unclosed → an issue
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let rc = togl_validate_sections(content.as_ptr(), 0, &mut out);
    assert_eq!(rc, 0);
    let json = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(v.is_array());
    togl_string_free(out);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p togl-ffi`
Expected: FAIL.

- [ ] **Step 3: Create `crates/togl-ffi/src/introspect.rs`**

```rust
//! Read-only introspection: results serialized to JSON strings.

use crate::error::TOGL_ERR_OPERATION;
use crate::mem::{borrow_str, guard, out_string};
use std::os::raw::{c_char, c_int};
use std::path::Path;
use togl_lib::core::{discover_sections, scan_sections, validate_sections};

/// Serializable projection of `togl_lib::core::SectionInfo` (which is not `Serialize`).
#[derive(serde::Serialize)]
struct SectionInfoJson {
    id: String,
    desc: Option<String>,
    start_line: usize,
    end_line: usize,
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, i32> {
    serde_json::to_string(value).map_err(|_| TOGL_ERR_OPERATION)
}

#[no_mangle]
pub extern "C" fn togl_discover_sections(
    content: *const c_char,
    out_json: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let sections: Vec<SectionInfoJson> = discover_sections(content)
            .into_iter()
            .map(|s| SectionInfoJson {
                id: s.id,
                desc: s.desc,
                start_line: s.start_line,
                end_line: s.end_line,
            })
            .collect();
        out_string(to_json(&sections)?, out_json)
    })
}

#[no_mangle]
pub extern "C" fn togl_scan_sections(
    path: *const c_char,
    content: *const c_char,
    out_json: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let path = borrow_str(path)?;
        let content = borrow_str(content)?;
        let sections = scan_sections(Path::new(path), content);
        out_string(to_json(&sections)?, out_json)
    })
}

#[no_mangle]
pub extern "C" fn togl_validate_sections(
    content: *const c_char,
    pair_only: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let sections = scan_sections(Path::new("<ffi>"), content);
        let issues = validate_sections(&[(std::path::PathBuf::from("<ffi>"), sections)], pair_only != 0);
        out_string(to_json(&issues)?, out_json)
    })
}
```

- [ ] **Step 4: Add `mod introspect;` to `lib.rs`** (after `mod transforms;`).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p togl-ffi`
Expected: PASS (11 tests). If the discover/scan fixtures don't produce sections, consult `crates/togl-lib/src/core.rs::discover_sections` for the exact marker syntax and fix the test fixtures.

- [ ] **Step 6: Commit**

```bash
git add crates/togl-ffi/
git commit -m "feat(ffi): introspection functions returning JSON (discover, scan, validate)"
```

---

### Task 5: cbindgen — generate and commit `include/togl.h`

**Files:**
- Create: `crates/togl-ffi/cbindgen.toml`, `crates/togl-ffi/build.rs`, `crates/togl-ffi/include/togl.h`

- [ ] **Step 1: Create `crates/togl-ffi/cbindgen.toml`**

```toml
language = "C"
include_guard = "TOGL_H"
pragma_once = true
cpp_compat = true
autogen_warning = "/* Generated by cbindgen. Do not edit by hand. Regenerated by build.rs. */"
documentation = true
style = "type"

[export]
prefix = ""
item_types = ["enums", "structs", "functions", "constants"]

[parse]
parse_deps = false
```

- [ ] **Step 2: Create `crates/togl-ffi/build.rs`**

```rust
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = crate_dir.join("include").join("togl.h");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    // Best-effort: don't fail the build if generation can't run (e.g. offline docs builds).
    if let Ok(builder) = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).unwrap())
        .generate()
    {
        std::fs::create_dir_all(crate_dir.join("include")).ok();
        builder.write_to_file(&out);
    }
}
```

- [ ] **Step 3: Generate the header**

Run: `cargo build -p togl-ffi`
Then: `test -f crates/togl-ffi/include/togl.h && echo OK`
Expected: `OK`, and the header declares `ToglRange`, the four lifecycle functions, the three transforms, and the three introspection functions.

- [ ] **Step 4: Verify the header content**

Run: `grep -cE 'togl_(version|abi_version|error_message|string_free|toggle_comments|find_and_toggle_section|activate_variant|discover_sections|scan_sections|validate_sections)' crates/togl-ffi/include/togl.h`
Expected: `10` (all ten exported functions present).

- [ ] **Step 5: Commit the generated header**

```bash
git add crates/togl-ffi/cbindgen.toml crates/togl-ffi/build.rs crates/togl-ffi/include/togl.h
git commit -m "feat(ffi): cbindgen build script and committed include/togl.h"
```

---

### Task 6: C smoke test (link libtogl, call it from C)

**Files:**
- Create: `crates/togl-ffi/tests/smoke.c`, `crates/togl-ffi/tests/c_smoke.rs`
- Modify: `crates/togl-ffi/Cargo.toml` (add `cc` dev-dependency)

- [ ] **Step 1: Create `crates/togl-ffi/tests/smoke.c`**

```c
#include "togl.h"
#include <assert.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* metadata */
    assert(togl_abi_version() == 1);
    assert(strlen(togl_version()) > 0);

    /* transform: comment line 1 */
    ToglRange ranges[1] = { { 1, 1 } };
    char *out = NULL;
    int rc = togl_toggle_comments("a\nb\n", ranges, 1, 1, &out);
    assert(rc == 0);
    assert(strstr(out, "# a") != NULL);
    togl_string_free(out);

    /* introspection: JSON array */
    char *json = NULL;
    rc = togl_discover_sections("# ID=foo\nx\n# ID=foo end\n", &json);
    assert(rc == 0);
    assert(json[0] == '[');
    togl_string_free(json);

    /* error path */
    char *bad = NULL;
    assert(togl_toggle_comments(NULL, NULL, 0, 0, &bad) == -1);

    printf("C smoke test passed\n");
    return 0;
}
```

- [ ] **Step 2: Add `cc` dev-dependency** to `crates/togl-ffi/Cargo.toml`:

```toml
[dev-dependencies]
cc = "1"
```

- [ ] **Step 3: Create `crates/togl-ffi/tests/c_smoke.rs`** — compiles `smoke.c`, links the static lib, runs it:

```rust
//! Compiles tests/smoke.c, links it against the freshly built libtogl static
//! archive, runs it, and asserts success. Proves the header + ABI + link work.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn c_program_links_and_runs() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("OUT_DIR"));
    // Static archive lives in target/<profile>/. OUT_DIR is target/<profile>/build/<pkg>/out.
    let target_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
    let lib = target_dir.join(if cfg!(windows) { "togl.lib" } else { "libtogl.a" });
    assert!(lib.exists(), "static lib not found at {:?} — run `cargo build` first", lib);

    let exe = out_dir.join("smoke_test_bin");
    let mut build = cc::Build::new();
    build.get_compiler();
    let compiler = cc::Build::new().get_compiler();
    let mut cmd = Command::new(compiler.path());
    cmd.arg(manifest.join("tests/smoke.c"))
        .arg("-I").arg(manifest.join("include"))
        .arg("-o").arg(&exe)
        .arg(&lib);
    // System libs the Rust staticlib needs at link time (pthread/dl/m on Unix).
    if !cfg!(windows) {
        cmd.args(["-lpthread", "-ldl", "-lm"]);
    }
    let status = cmd.status().expect("failed to invoke C compiler");
    assert!(status.success(), "C compile/link failed");

    let run = Command::new(&exe).status().expect("failed to run smoke test");
    assert!(run.success(), "C smoke test returned failure");
}
```

- [ ] **Step 4: Build then run the smoke test**

Run: `cargo build -p togl-ffi && cargo test -p togl-ffi --test c_smoke -- --nocapture`
Expected: PASS, output contains `C smoke test passed`.
If linking fails for missing system symbols, add the reported library to the `-l` list (Linux typically needs `-lpthread -ldl -lm`; macOS usually needs none of these — guard already skips on Windows only, so on macOS the extra `-l` flags are harmless/ignored if present, but remove any that error).

- [ ] **Step 5: Commit**

```bash
git add crates/togl-ffi/Cargo.toml crates/togl-ffi/tests/
git commit -m "test(ffi): C smoke test linking and exercising libtogl"
```

---

### Task 7: pkg-config file

**Files:**
- Create: `crates/togl-ffi/togl.pc.in`
- Create: `crates/togl-ffi/README.md` (usage)

- [ ] **Step 1: Create `crates/togl-ffi/togl.pc.in`** (template; install tooling substitutes `@prefix@`/`@version@`):

```
prefix=@prefix@
exec_prefix=${prefix}
libdir=${exec_prefix}/lib
includedir=${prefix}/include

Name: togl
Description: ABI-stable C library for toggling code comments (libtogl)
Version: @version@
Libs: -L${libdir} -ltogl
Cflags: -I${includedir}
```

- [ ] **Step 2: Create `crates/togl-ffi/README.md`** documenting: the C API, the `togl_string_free` ownership rule, `force_mode`/`comment_marker`/`pair_only` encodings, and a minimal C usage example (mirror `tests/smoke.c`).

- [ ] **Step 3: Verify the template is well-formed**

Run: `grep -c '@version@' crates/togl-ffi/togl.pc.in`
Expected: `1`.

- [ ] **Step 4: Commit**

```bash
git add crates/togl-ffi/togl.pc.in crates/togl-ffi/README.md
git commit -m "feat(ffi): pkg-config template and C API README"
```

---

### Task 8: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Clean workspace build**

Run: `cargo build --workspace --all-targets`
Expected: PASS (all three crates).

- [ ] **Step 2: Full test suite (workspace)**

Run: `cargo test --workspace --all-features`
Expected: PASS — `togl-lib` and `togl-cli` unchanged from baseline; `togl-ffi` unit tests + C smoke test pass.

- [ ] **Step 3: Lint + format**

Run: `cargo clippy --workspace --all-targets -- -D warnings` then `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4: Confirm both library kinds and header exist**

Run: `ls target/debug | grep -E 'libtogl\.(a|so|dylib)'` and `test -f crates/togl-ffi/include/togl.h && echo header-ok`
Expected: static + shared present; `header-ok`.

- [ ] **Step 5: Update PROJECTS.md** with a `togl-ffi` project entry (next free ID, this repo's legend) and commit.

---

## Self-Review

**Spec coverage:** workspace integration §3.1 → Task 1 ✓; ABI conventions §5 (status codes, panic guard, string ownership, no structs but ToglRange, abi_version) → Task 2 + ToglRange in Task 3 ✓; C API §4 transforms → Task 3 ✓, introspection JSON → Task 4 ✓; cbindgen + committed header §6 → Task 5 ✓; C smoke test §8 → Task 6 ✓; pkg-config §7 → Task 7 ✓; testing §8 → Tasks 2-6 + Task 8 ✓. Flake/nixpkgs packaging (§7/§9) intentionally deferred (parked follow-on) — noted in plan goal.

**Placeholder scan:** test-fixture fallback notes (Task 3/4/6) give concrete remediation (consult named files / adjust `-l` flags), not vague "handle errors". No TBD/TODO.

**Type consistency:** `ToglRange{start,end}`, error constants (`TOGL_ERR_*`), `borrow_str`/`out_string`/`guard`, `force_str`, `comment_style`, `SectionInfoJson` are defined once and referenced consistently. Lib calls match real signatures: `toggle_comments(&str,&[LineRange],Option<&str>)`, `find_and_toggle_section(&mut [String],&str,&Option<String>,&CommentStyle)`, `activate_variant(&str,&str,&str,&CommentStyle)`, `discover_sections(&str)`, `scan_sections(&Path,&str)`, `validate_sections(&[(PathBuf,Vec<ScanSectionInfo>)],bool)`.
