use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use toggle::walk::{collect_files, WalkOptions};

fn default_opts() -> WalkOptions {
    WalkOptions::default()
}

#[test]
fn test_collect_files_single_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.py");
    fs::write(&file, "print('hello')\n").unwrap();

    let files = collect_files(std::slice::from_ref(&file), false, &default_opts()).unwrap();
    assert_eq!(files, vec![file]);
}

#[test]
fn test_collect_files_directory_recursive() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let f1 = dir.path().join("a.py");
    let f2 = sub.join("b.rs");
    fs::write(&f1, "").unwrap();
    fs::write(&f2, "").unwrap();

    let files = collect_files(&[dir.path().to_path_buf()], true, &default_opts()).unwrap();
    assert!(files.contains(&f1));
    assert!(files.contains(&f2));
    assert_eq!(files.len(), 2);
}

#[test]
fn test_collect_files_directory_non_recursive_errors() {
    let dir = TempDir::new().unwrap();
    let result = collect_files(&[dir.path().to_path_buf()], false, &default_opts());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("directory"),
        "Error should mention directory: {}",
        err
    );
}

#[test]
fn test_collect_files_skips_hidden_dirs() {
    let dir = TempDir::new().unwrap();
    let hidden = dir.path().join(".hidden");
    fs::create_dir(&hidden).unwrap();
    let visible = dir.path().join("visible");
    fs::create_dir(&visible).unwrap();
    fs::write(hidden.join("secret.py"), "").unwrap();
    fs::write(visible.join("public.py"), "").unwrap();

    let files = collect_files(&[dir.path().to_path_buf()], true, &default_opts()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("public.py"));
}

#[test]
fn test_collect_files_skips_node_modules() {
    let dir = TempDir::new().unwrap();
    let nm = dir.path().join("node_modules");
    fs::create_dir(&nm).unwrap();
    fs::write(nm.join("pkg.js"), "").unwrap();
    fs::write(dir.path().join("app.js"), "").unwrap();

    let files = collect_files(&[dir.path().to_path_buf()], true, &default_opts()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("app.js"));
}

#[test]
fn test_collect_files_skips_unsupported_extensions() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("image.png"), "").unwrap();
    fs::write(dir.path().join("data.bin"), "").unwrap();
    fs::write(dir.path().join("readme.md"), "").unwrap();
    fs::write(dir.path().join("code.py"), "").unwrap();

    let files = collect_files(&[dir.path().to_path_buf()], true, &default_opts()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("code.py"));
}

#[test]
fn test_collect_files_mixed_paths() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("lib");
    fs::create_dir(&sub).unwrap();
    let standalone = dir.path().join("main.py");
    fs::write(&standalone, "").unwrap();
    fs::write(sub.join("util.py"), "").unwrap();

    let files = collect_files(&[standalone.clone(), sub.clone()], true, &default_opts()).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn test_collect_files_nonexistent_passes_through() {
    let path = PathBuf::from("/tmp/definitely_does_not_exist_toggle_test.py");
    // Nonexistent file paths pass through for downstream error handling
    let files = collect_files(std::slice::from_ref(&path), false, &default_opts()).unwrap();
    assert_eq!(files, vec![path]);
}

#[test]
fn test_collect_files_sorted_deterministic() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("z.py"), "").unwrap();
    fs::write(dir.path().join("a.py"), "").unwrap();
    fs::write(dir.path().join("m.py"), "").unwrap();

    let files = collect_files(&[dir.path().to_path_buf()], true, &default_opts()).unwrap();
    let names: Vec<&str> = files
        .iter()
        .map(|f| f.file_name().unwrap().to_str().unwrap())
        .collect();
    assert_eq!(names, vec!["a.py", "m.py", "z.py"]);
}

#[test]
fn test_collect_files_max_depth() {
    let dir = TempDir::new().unwrap();
    let deep = dir.path().join("a").join("b");
    fs::create_dir_all(&deep).unwrap();
    fs::write(dir.path().join("top.py"), "").unwrap();
    fs::write(dir.path().join("a").join("mid.py"), "").unwrap();
    fs::write(deep.join("deep.py"), "").unwrap();

    let opts = WalkOptions {
        skip_hidden: true,
        max_depth: Some(2), // root + 1 level
        verbose: false,
    };
    let files = collect_files(&[dir.path().to_path_buf()], true, &opts).unwrap();
    assert!(files.iter().any(|f| f.ends_with("top.py")));
    assert!(files.iter().any(|f| f.ends_with("mid.py")));
    assert!(!files.iter().any(|f| f.ends_with("deep.py")));
}
