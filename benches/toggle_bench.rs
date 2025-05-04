use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use toggle::core::{LineRange, toggle_comments};
use toggle::toggle;

// Create a fixture file with alternating commented and uncommented lines
fn create_fixture_file(path: &Path, num_lines: usize) -> io::Result<()> {
    let mut file = File::create(path)?;
    
    for i in 0..num_lines {
        if i % 2 == 0 {
            writeln!(file, "# This is a commented line {}", i + 1)?;
        } else {
            writeln!(file, "print('This is an uncommented line {}')", i + 1)?;
        }
    }
    
    Ok(())
}

// Read the entire contents of a file
fn read_file_contents(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn bench_toggle_comments(c: &mut Criterion) {
    let fixture_path = Path::new("benches/fixture_1000.py");
    
    // Only create the fixture file if it doesn't exist
    if !fixture_path.exists() {
        create_fixture_file(fixture_path, 1000).expect("Failed to create fixture file");
    }
    
    let content = read_file_contents(fixture_path).expect("Failed to read fixture file");
    
    // Benchmark toggling the entire file
    c.bench_function("toggle_all", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 1000)];
            toggle_comments(black_box(&content), black_box(&ranges), None)
        })
    });
    
    // Benchmark toggling specific ranges (first 100 lines)
    c.bench_function("toggle_range_100", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 100)];
            toggle_comments(black_box(&content), black_box(&ranges), None)
        })
    });
}

fn toggle_benchmark(c: &mut Criterion) {
    c.bench_function("toggle empty", |b| {
        b.iter(|| toggle(&["test.py".to_string()]))
    });
}

criterion_group!(benches, bench_toggle_comments, toggle_benchmark);
criterion_main!(benches); 