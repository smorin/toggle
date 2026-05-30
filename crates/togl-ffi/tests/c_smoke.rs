//! Compiles tests/smoke.c, links it against the freshly built `libtogl` static
//! archive, runs it, and asserts success. Proves the header + ABI + link work
//! end to end from a real C program.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn c_program_links_and_runs() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // The test executable lives at target/<profile>/deps/<bin>; the static
    // archive `libtogl.a` is produced two directories up, in target/<profile>/.
    let test_exe = std::env::current_exe().expect("current_exe");
    let profile_dir = test_exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile> dir");
    let lib = profile_dir.join("libtogl.a");
    // `cargo test` builds only the rlib needed to run unit tests, not the
    // staticlib/cdylib artifacts. Produce `libtogl.a` explicitly if absent so
    // this test is self-sufficient under a bare `cargo test` (as CI runs it).
    if !lib.exists() {
        let mut args = vec!["build", "-p", "togl-ffi"];
        if profile_dir.file_name().and_then(|n| n.to_str()) == Some("release") {
            args.push("--release");
        }
        let status = Command::new(env!("CARGO"))
            .args(&args)
            .status()
            .expect("failed to run cargo build for libtogl");
        assert!(status.success(), "cargo build -p togl-ffi failed");
    }
    assert!(lib.exists(), "static lib still not found at {lib:?}");

    let exe = std::env::temp_dir().join("togl_c_smoke_bin");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let mut cmd = Command::new(&cc);
    cmd.arg(manifest.join("tests/smoke.c"))
        .arg("-I")
        .arg(manifest.join("include"))
        .arg("-o")
        .arg(&exe)
        .arg(&lib);
    // System libraries the Rust staticlib needs at link time on Linux.
    if cfg!(target_os = "linux") {
        cmd.args(["-lpthread", "-ldl", "-lm"]);
    }
    let status = cmd.status().expect("failed to invoke C compiler");
    assert!(status.success(), "C compile/link failed");

    let run = Command::new(&exe)
        .status()
        .expect("failed to run smoke test");
    assert!(run.success(), "C smoke test returned failure");
}
