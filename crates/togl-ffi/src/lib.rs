//! C ABI for togl (`libtogl`). All exported symbols are `togl_`-prefixed,
//! return an `int` status, and never unwind across the FFI boundary.

/// ABI version for runtime negotiation. Bump on any breaking C-ABI change.
pub const TOGL_ABI_VERSION: u32 = 1;
