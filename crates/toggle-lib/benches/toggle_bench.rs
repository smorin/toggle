use criterion::{black_box, criterion_group, criterion_main, Criterion};
use toggle_lib::core::{
    find_and_toggle_section, toggle_comments, toggle_comments_multi, CommentStyle, LineRange,
};

/// Generate fixture content with alternating commented and uncommented lines.
fn generate_fixture(num_lines: usize) -> String {
    let mut buf = String::new();
    for i in 0..num_lines {
        if i % 2 == 0 {
            buf.push_str(&format!("# This is a commented line {}\n", i + 1));
        } else {
            buf.push_str(&format!("print('This is an uncommented line {}')\n", i + 1));
        }
    }
    buf
}

/// Generate content with section markers for section-toggle benchmarks.
fn generate_section_content(num_sections: usize, lines_per_section: usize) -> String {
    let mut buf = String::new();
    for s in 0..num_sections {
        buf.push_str(&format!("# toggle:start ID=section{}\n", s));
        for l in 0..lines_per_section {
            buf.push_str(&format!("print('section {} line {}')\n", s, l + 1));
        }
        buf.push_str(&format!("# toggle:end ID=section{}\n", s));
    }
    buf
}

// ── Single-file toggle at various sizes ─────────────────────────────────────

fn bench_toggle_by_size(c: &mut Criterion) {
    let small = generate_fixture(50);
    let medium = generate_fixture(500);
    let large = generate_fixture(5000);

    c.bench_function("toggle_small_50", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 50)];
            toggle_comments(black_box(&small), black_box(&ranges), None)
        })
    });

    c.bench_function("toggle_medium_500", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 500)];
            toggle_comments(black_box(&medium), black_box(&ranges), None)
        })
    });

    c.bench_function("toggle_large_5000", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 5000)];
            toggle_comments(black_box(&large), black_box(&ranges), None)
        })
    });
}

// ── Original 1000-line benchmarks ───────────────────────────────────────────

fn bench_toggle_comments(c: &mut Criterion) {
    let content = generate_fixture(1000);

    c.bench_function("toggle_all_1000", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 1000)];
            toggle_comments(black_box(&content), black_box(&ranges), None)
        })
    });

    c.bench_function("toggle_range_100", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 100)];
            toggle_comments(black_box(&content), black_box(&ranges), None)
        })
    });
}

// ── Multi-line (block) comment toggle ───────────────────────────────────────

fn bench_toggle_multi(c: &mut Criterion) {
    // Generate JS-style content for block comments
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("console.log('line {}');\n", i + 1));
    }

    c.bench_function("toggle_multi_1000", |b| {
        b.iter(|| {
            let ranges = vec![LineRange::new(1, 1000)];
            toggle_comments_multi(black_box(&content), black_box(&ranges), None, "/*", "*/")
        })
    });
}

// ── Section discovery and toggle ────────────────────────────────────────────

fn bench_section_toggle(c: &mut Criterion) {
    let content = generate_section_content(20, 10);
    let style = CommentStyle {
        single_line: "#".to_string(),
        multi_line_start: None,
        multi_line_end: None,
    };

    c.bench_function("section_toggle_single", |b| {
        b.iter(|| {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            find_and_toggle_section(black_box(&mut lines), black_box("section10"), &None, &style)
                .unwrap()
        })
    });

    c.bench_function("section_toggle_all_20", |b| {
        b.iter(|| {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            for i in 0..20 {
                let id = format!("section{}", i);
                find_and_toggle_section(black_box(&mut lines), &id, &None, &style).unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    bench_toggle_by_size,
    bench_toggle_comments,
    bench_toggle_multi,
    bench_section_toggle,
);
criterion_main!(benches);
