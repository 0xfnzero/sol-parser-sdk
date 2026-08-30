use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::logs::optimized_matcher::detect_pumpfun_create;
use std::hint::black_box;

fn logs_without_create() -> Vec<String> {
    (0..64)
        .map(|index| {
            format!("Program log: instruction {index} consumed 12345 of 200000 compute units")
        })
        .collect()
}

fn benchmark(c: &mut Criterion) {
    let logs = logs_without_create();
    c.bench_function("pumpfun_create_detection/64_logs_transaction_path", |b| {
        b.iter(|| black_box(detect_pumpfun_create(black_box(&logs))))
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
