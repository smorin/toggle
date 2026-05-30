use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = crate_dir.join("include").join("togl.h");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).unwrap();
    // Best-effort: don't fail the build if generation can't run (e.g. offline docs builds).
    if let Ok(bindings) = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        std::fs::create_dir_all(crate_dir.join("include")).ok();
        bindings.write_to_file(&out);
    }
}
