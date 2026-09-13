use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hybrid_format::hformat;

fn format_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("constant");
    group.bench_function("std", |b| {
        b.iter(|| format!("Hello, world!"));
    });
    group.bench_function("hybridformat", |b| {
        b.iter(|| hformat!("Hello, world!"));
    });
    group.finish();

    let mut group = c.benchmark_group("all_dynamic");
    let n = 42;
    let f = 42.676767;
    let name = "Horace";
    group.bench_function("std", |b| {
        b.iter(|| {
            format!(
                "Hello {}, n={}, f={}",
                black_box(name),
                black_box(n),
                black_box(f)
            )
        })
    });
    group.bench_function("hybridformat", |b| {
        b.iter(|| {
            hformat!(
                "Hello {}, n={}, f={}",
                black_box(name),
                black_box(n),
                black_box(f)
            )
        })
    });
    group.finish();

    let mut group = c.benchmark_group("ten_const_ten_dynamic");
    let a = 4.2e10;
    let b1 = 6.7e10;
    let c = true;
    let d = false;
    let e = 42 * 1000;
    let f = "Hello,";
    let g = ", world!";
    let h = 'x';
    let i = false;
    let j = "123456789123456789123456789123456789";
    const A: u8 = 42;
    const B: u8 = 67;
    // const A: f64 = 4.2e10;
    // const B: f64 = 6.7e10;
    const C: bool = true;
    const D: bool = false;
    const E: i32 = 42 * 1000;
    const F: &str = "Hello,";
    const G: &str = ", world!";
    const H: char = 'x';
    const I: bool = false;
    const J: &str = "123456789123456789123456789123456789";
    group.bench_function("std", |b| {
        b.iter(|| {
            format!(
                "{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}",
                black_box(a),
                black_box(b1),
                black_box(c),
                black_box(d),
                black_box(e),
                black_box(f),
                black_box(g),
                black_box(h),
                black_box(i),
                black_box(j),
                A,
                B,
                C,
                D,
                E,
                F,
                G,
                H,
                I,
                J
            )
        })
    });
    group.bench_function("hybridformat", |b| {
        b.iter(|| {
            hformat!(
                "{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}",
                black_box(a),
                black_box(b1),
                black_box(c),
                black_box(d),
                black_box(e),
                black_box(f),
                black_box(g),
                black_box(h),
                black_box(i),
                black_box(j),
                A,
                B,
                C,
                D,
                E,
                F,
                G,
                H,
                I,
                J
            )
        })
    });
    group.finish();
}

criterion_group!(benches, format_benchmarks);
criterion_main!(benches);
