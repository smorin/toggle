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

/// Toggle line-comment markers on the given 1-based inclusive ranges.
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
            slice
                .iter()
                .map(|r| LineRange::new(r.start, r.end))
                .collect()
        };
        let result = toggle_comments(content, &ranges, force);
        out_string(result, out_result)
    })
}

/// Toggle the comment state of a named `toggle:start`/`toggle:end` section.
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

/// Activate one variant of a group, commenting out its siblings.
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
