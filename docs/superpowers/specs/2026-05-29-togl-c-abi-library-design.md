# Design: ABI-Stable C Library for togl (static + shared)

**Date:** 2026-05-29
**Status:** Approved (design); pending spec review
**Author:** Steve Morin (with Claude)

## 1. Goal

Have the `togl`/`toggle` project additionally produce an **ABI-stable C library**
(`libtogl`) in both **static** (`.a`) and **shared** (`.so`/`.dylib`) forms, so it can be:

1. A general-purpose C library linked directly by external C/C++ (or any FFI-capable) consumers, and
2. The single shared Rust core that the project's future Python and TypeScript libraries bind to.

The C surface exposes the **string-core + read-only introspection** subset of the existing
`toggle` library — content-in/content-out transforms plus JSON-returning analysis. No
filesystem operations cross the C boundary.

## 2. Locked decisions

| Decision | Choice |
|---|---|
| Purpose | General-purpose: external C consumers **and** the core for future Python/TS bindings |
| API scope | String core (transforms) + read-only introspection (JSON results) |
| Crate structure | **Virtual Cargo workspace** at repo root; existing crate moves to `crates/togl/`; new `crates/togl-ffi/` produces `libtogl` |
| Header generation | **cbindgen** at build time, **and** the generated header is committed to the repo |
| nixpkgs packaging | C library is a **separate** nixpkgs package (`libtogl`), landing **after** the CLI (`togl`) PR |

## 3. Architecture

### 3.1 Virtual workspace migration

Root `Cargo.toml` becomes a **virtual workspace manifest** (only `[workspace]`, no `[package]`).
The current single crate moves into a member directory; a second member provides the C ABI.

```
toggle/                       # repo root = virtual workspace
├── Cargo.toml                # [workspace] members = ["crates/togl", "crates/togl-ffi"]
├── Cargo.lock                # STAYS at root (workspace lockfile)
├── README.md, Justfile, Makefile, lefthook.yml, deny.toml, CHANGELOG.md  # repo-level, stay
├── crates/
│   ├── togl/                 # the existing crate, moved verbatim
│   │   ├── Cargo.toml        # [package] name="togl"; [lib] toggle; [[bin]] toggle, togl
│   │   ├── src/              # moved from ./src
│   │   ├── tests/            # moved from ./tests
│   │   └── benches/          # moved from ./benches
│   └── togl-ffi/             # NEW C ABI crate → libtogl
│       ├── Cargo.toml        # crate-type = ["staticlib","cdylib"]; deps: toggle = { path = "../togl" }
│       ├── build.rs          # runs cbindgen → include/togl.h
│       ├── cbindgen.toml
│       ├── include/togl.h    # generated AND committed
│       └── src/lib.rs        # extern "C" surface
```

### 3.2 Migration touchpoints (must be updated as part of the move)

- **Justfile:** `cargo run -- {{args}}` and `cargo run --release -- {{args}}` become ambiguous in a
  multi-package workspace → add `-p togl` (or `--bin toggle`). Other recipes (`cargo fmt --all`,
  `cargo test`, `cargo build`, `cargo clippy`) are workspace-aware and unaffected.
- **In-flight nixpkgs `pkgs/by-name/to/togl/package.nix`:** `src` is now a workspace root, so add
  `cargoBuildFlags = [ "-p" "togl" ]` (and `cargoTestFlags`/`buildAndTestSubdir` as needed) so it
  builds only the CLI package. `cargoHash` will change (workspace `Cargo.lock`).
- **crates.io publishing:** `togl` now publishes from `crates/togl/`. The crate's `readme`/`license`
  paths are relative to the crate dir; verify `readme = "README.md"` still resolves (move a copy
  into the crate, or point at the repo README via included file). Confirm `cargo publish -p togl`.
- **Release workflow (`.github/workflows/release.yml`) and CI (`ci.yml`, `audit.yml`):** verify any
  hardcoded `src/` / manifest paths; update to workspace-aware invocations.
- **PROJECTS.md:** add a project entry for this work (per repo convention).

### 3.3 `togl-ffi` crate

- Depends on `toggle` (the moved lib) via path dependency.
- `crate-type = ["staticlib", "cdylib"]` → emits `libtogl.a` and `libtogl.so`/`.dylib`.
- All exposed symbols prefixed `togl_`.
- `build.rs` invokes cbindgen to (re)generate `include/togl.h`; the file is also committed so
  non-Nix consumers and code review see the C API without building.

## 4. C API surface

### 4.1 Transforms (content in → new content out)

```c
int togl_toggle_comments(const char *content, const ToglRange *ranges,
                         size_t range_count, int force_mode, char **out_result);
int togl_find_and_toggle_section(const char *content, const char *section_id,
                                 char **out_result);
int togl_activate_variant(const char *content, const char *group,
                          const char *variant, char **out_result);
```

### 4.2 Introspection (→ JSON string)

```c
int togl_discover_sections(const char *content, char **out_json);
int togl_scan_sections(const char *path, const char *content, char **out_json);
int togl_validate_sections(const char *content, int check_level, char **out_json);
```

Note: `path` in `togl_scan_sections` is a **label only** — it is echoed into the result JSON for
identification and is **never opened or read**. The caller supplies file contents via `content`.
This preserves the "no filesystem in the C layer" rule.

### 4.3 Lifecycle / metadata

```c
void        togl_string_free(char *s);     // free any char* returned by the lib
const char *togl_version(void);            // crate version, static, do NOT free
uint32_t    togl_abi_version(void);        // integer ABI version for runtime negotiation
const char *togl_error_message(int code);  // human-readable text for an error code
```

### 4.4 The one boundary struct

```c
typedef struct { size_t start; size_t end; } ToglRange;  // frozen POD, never changed
```

## 5. ABI-stability conventions

- **Return codes:** every function returns `int` (`0` = success, negative = stable error-code enum).
  Results are delivered through out-pointers. Error codes, once assigned, never change meaning.
- **Panic safety:** every `extern "C"` function body is wrapped in `std::panic::catch_unwind`; a
  caught panic maps to a dedicated error code. No unwinding crosses the FFI boundary.
- **Memory ownership:** every `char*` the library returns is library-owned and freed **only** via
  `togl_string_free`. Callers never call C `free()` on it (Rust allocator ≠ C allocator).
  `togl_version` returns a static pointer that is never freed.
- **No Rust structs on the boundary** except the frozen `ToglRange`. All complex/aggregate results
  are serialized to JSON strings, so the internal Rust types (`SectionInfo`, `ScanSectionInfo`, …)
  can evolve freely without breaking compiled consumers.
- **Versioning:** `togl_abi_version()` exposes an integer for runtime negotiation; the shared
  library is built with a proper `SONAME`/version so the dynamic linker enforces compatibility.

## 6. Header generation — cbindgen

- `cbindgen.toml` configures: C (not C++) output, `togl_` symbol prefix, include guard, the frozen
  `ToglRange`, and pinned function ordering for readable diffs.
- `build.rs` regenerates `include/togl.h` on build; the generated header is **committed** and a CI
  check fails if the committed header drifts from a fresh generation.

## 7. Build artifacts & pkg-config

- Static (`libtogl.a`) and shared (`libtogl.so` / `.dylib`) from the single `togl-ffi` crate.
- A **pkg-config** file (`togl.pc`) is produced/installed so consumers use
  `pkg-config --cflags --libs togl`.

## 8. Testing

- **Rust unit tests** that call the `extern "C"` functions directly: round-trip a string through
  `togl_toggle_comments`; assert `togl_discover_sections` returns well-formed JSON; assert error
  codes for bad input (null pointer, invalid UTF-8, bad range); assert `togl_string_free` frees
  returned buffers without double-free.
- **C smoke test:** a small `.c` program that `#include "togl.h"`, links `libtogl`, calls a
  transform and an introspection function, and verifies output — proving header + link + ABI work
  end to end.
- **CI matrix:** build and run both static and shared targets on Linux and macOS; run the
  header-drift check.

## 9. Packaging

- **Repo flake (Track A):** expose `packages.libtogl` (the C library, multi-output) alongside
  `packages.togl` (the CLI), both built from the workspace.
- **nixpkgs (Track B):** a **separate** package `libtogl` in its own `pkgs/by-name/` dir, built from
  the same source with `cargoBuildFlags = [ "-p" "togl-ffi" ]`, multi-output:
  - `out` → shared library (`.so`/`.dylib`)
  - `dev` → `togl.h`, static `libtogl.a`, and `togl.pc`
  This lands as a **follow-on PR after** the CLI `togl` package PR, to keep each PR small.

## 10. Out of scope (YAGNI)

- Filesystem operations in the C layer (callers do their own I/O).
- The Python and TypeScript bindings themselves (separate future projects that *consume* `libtogl`).
- Exposing the full Rust API — only the string-core + introspection subset is in scope.
- C++-idiomatic wrappers, async, or callback interfaces.

## 11. Risks / open items for the plan

- Confirm `cargo publish -p togl` still works from the workspace (readme/license path resolution).
- Decide exact `force_mode` and `check_level` integer encodings (stable enums) during planning.
- Confirm macOS `SONAME`/install-name handling for the `.dylib` in both flake and nixpkgs builds.
- Verify the in-flight nixpkgs CLI PR is updated for the workspace `src` before it merges, so the
  two PRs stay consistent.
