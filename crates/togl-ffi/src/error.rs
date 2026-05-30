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
