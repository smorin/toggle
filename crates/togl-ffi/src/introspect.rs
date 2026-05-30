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

/// Discover sections in `content`; returns a JSON array of `{id,desc,start_line,end_line}`.
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

/// Scan `content` (labeled by `path`); returns a JSON array of scan-section records.
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

/// Validate `content`; returns a JSON array of check issues. `pair_only != 0` enables pair mode.
#[no_mangle]
pub extern "C" fn togl_validate_sections(
    content: *const c_char,
    pair_only: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    guard(|| {
        let content = borrow_str(content)?;
        let sections = scan_sections(Path::new("<ffi>"), content);
        let issues = validate_sections(
            &[(std::path::PathBuf::from("<ffi>"), sections)],
            pair_only != 0,
        );
        out_string(to_json(&issues)?, out_json)
    })
}
