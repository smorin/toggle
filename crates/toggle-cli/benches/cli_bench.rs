use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn create_fixture_file(path: &Path, num_lines: usize) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    for i in 0..num_lines {
        if i % 2 == 0 {
            writeln!(file, "# This is a commented line {}", i + 1)?;
        } else {
            writeln!(file, "print('This is an uncommented line {}')", i + 1)?;
        }
    }
    Ok(())
}

fn bench_cli_100_files(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut paths: Vec<String> = Vec::new();

    for i in 0..100 {
        let file_path = dir.path().join(format!("file_{}.py", i));
        create_fixture_file(&file_path, 50).expect("Failed to create fixture file");
        paths.push(file_path.to_str().unwrap().to_string());
    }

    let binary = env!("CARGO_BIN_EXE_toggle");

    c.bench_function("cli_100_files", |b| {
        b.iter_batched(
            || {
                for (i, p) in paths.iter().enumerate() {
                    create_fixture_file(Path::new(p), 50)
                        .unwrap_or_else(|_| panic!("Failed to recreate file {}", i));
                }
            },
            |()| {
                let mut cmd = std::process::Command::new(binary);
                cmd.args(&paths).args(["-l", "1:10"]);
                let output = cmd.output().expect("Failed to run toggle");
                assert!(output.status.success(), "toggle exited with error");
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_cli_100_files);
criterion_main!(benches);
