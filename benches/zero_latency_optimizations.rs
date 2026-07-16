//! Zero-Latency Optimization Benchmarks
//!
//! Benchmarks to measure the performance impact of various optimizations:
//! - SmallVec stack allocation vs Vec heap allocation
//! - Zero-copy string slicing vs String allocation
//! - Discriminator LUT lookup
//! - Hot-path fast routing
//!
//! Run with: cargo bench --bench zero_latency_optimizations

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use smallvec::SmallVec;
use sol_parser_sdk::logs::utils::{
    read_string, read_string_ref,
    text_parser::{extract_text_field, extract_text_field_ref},
};
use std::hint::black_box;

// ========================================================================
// SmallVec vs Vec Benchmarks
// ========================================================================

fn bench_smallvec_stack_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("SmallVec Stack Allocation");

    // Test different sizes to show when stack allocation is beneficial
    for size in [1, 2, 3, 4, 5, 8, 12].iter() {
        group.bench_with_input(BenchmarkId::new("SmallVec", size), size, |b, &size| {
            b.iter(|| {
                let mut events: SmallVec<[u64; 4]> = SmallVec::new();
                for i in 0..size {
                    events.push(black_box(i));
                }
                black_box(events)
            });
        });

        group.bench_with_input(BenchmarkId::new("Vec", size), size, |b, &size| {
            b.iter(|| {
                let mut events: Vec<u64> = Vec::new();
                for i in 0..size {
                    events.push(black_box(i));
                }
                black_box(events)
            });
        });
    }

    group.finish();
}

// ========================================================================
// Zero-Copy String Benchmarks
// ========================================================================

fn bench_zero_copy_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("Zero-Copy String Parsing");

    // Test data: length-prefixed string "SOL"
    let short_string_data: Vec<u8> = vec![3, 0, 0, 0, b'S', b'O', b'L'];

    // Test data: length-prefixed string with 20 chars
    let medium_string_data: Vec<u8> = {
        let mut data = vec![20, 0, 0, 0];
        data.extend_from_slice(b"PUMP_TOKEN_123456789");
        data
    };

    // Test data: length-prefixed string with 64 chars
    let long_string_data: Vec<u8> = {
        let mut data = vec![64, 0, 0, 0];
        data.extend_from_slice(b"https://arweave.net/abcdefghijklmnopqrstuvwxyz1234567890ABCDEFGH");
        data
    };

    // Benchmark short string (3 bytes)
    group.bench_function("read_string_ref (3 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string_ref(black_box(&short_string_data), 0).unwrap();
            black_box(s)
        });
    });

    group.bench_function("read_string (3 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string(black_box(&short_string_data), 0).unwrap();
            black_box(s)
        });
    });

    // Benchmark medium string (20 bytes)
    group.bench_function("read_string_ref (20 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string_ref(black_box(&medium_string_data), 0).unwrap();
            black_box(s)
        });
    });

    group.bench_function("read_string (20 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string(black_box(&medium_string_data), 0).unwrap();
            black_box(s)
        });
    });

    // Benchmark long string (64 bytes)
    group.bench_function("read_string_ref (64 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string_ref(black_box(&long_string_data), 0).unwrap();
            black_box(s)
        });
    });

    group.bench_function("read_string (64 bytes)", |b| {
        b.iter(|| {
            let (s, _) = read_string(black_box(&long_string_data), 0).unwrap();
            black_box(s)
        });
    });

    group.finish();
}

fn bench_text_field_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text Field Extraction");

    let log_text = "amount: 1000000, user: ABC123, pool: XYZ789, timestamp: 1234567890";

    group.bench_function("extract_text_field_ref", |b| {
        b.iter(|| {
            let amount = extract_text_field_ref(black_box(log_text), "amount").unwrap();
            let user = extract_text_field_ref(black_box(log_text), "user").unwrap();
            let pool = extract_text_field_ref(black_box(log_text), "pool").unwrap();
            black_box((amount, user, pool))
        });
    });

    group.bench_function("extract_text_field", |b| {
        b.iter(|| {
            let amount = extract_text_field(black_box(log_text), "amount").unwrap();
            let user = extract_text_field(black_box(log_text), "user").unwrap();
            let pool = extract_text_field(black_box(log_text), "pool").unwrap();
            black_box((amount, user, pool))
        });
    });

    group.finish();
}

// ========================================================================
// Discriminator Lookup Benchmarks
// ========================================================================

fn bench_discriminator_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("Discriminator Lookup");

    // Test hot-path discriminators (should be fast)
    let hot_discriminators = [
        0xEE61E64ED37FDBBD, // PumpFun Trade
        0xC887759EE19EC6F8, // Raydium CLMM Swap
        0x0900000000000000, // Raydium AMM Swap Base In
        0x7777F52C1F52F467, // PumpSwap Buy
        0x2ADC03A50A372F3E, // PumpSwap Sell
    ];

    // Test cold-path discriminators (handled by match)
    let cold_discriminators = [
        0x0A00000000000000, // Some cold discriminator
        0x0B00000000000000, // Another cold discriminator
    ];

    group.bench_function("Hot-path discriminator (PumpFun Trade)", |b| {
        b.iter(|| {
            // Simulate hot-path check
            let disc = black_box(hot_discriminators[0]);
            if disc == 0xEE61E64ED37FDBBD_u64 {
                black_box(1)
            } else {
                black_box(0)
            }
        });
    });

    group.bench_function("Hot-path discriminator (Raydium CLMM)", |b| {
        b.iter(|| {
            let disc = black_box(hot_discriminators[1]);
            if disc == 0xEE61E64ED37FDBBD_u64 {
                black_box(1)
            } else if disc == 0xC887759EE19EC6F8_u64 {
                black_box(2)
            } else {
                black_box(0)
            }
        });
    });

    group.bench_function("Cold-path discriminator (match)", |b| {
        b.iter(|| {
            let disc = black_box(cold_discriminators[0]);
            match disc {
                0xEE61E64ED37FDBBD_u64 => black_box(1),
                0xC887759EE19EC6F8_u64 => black_box(2),
                0x0900000000000000_u64 => black_box(3),
                0x7777F52C1F52F467_u64 => black_box(4),
                0x2ADC03A50A372F3E_u64 => black_box(5),
                0x0A00000000000000_u64 => black_box(6),
                0x0B00000000000000_u64 => black_box(7),
                _ => black_box(0),
            }
        });
    });

    group.finish();
}

// ========================================================================
// Branch Prediction Benchmarks
// ========================================================================

/// Simulate likely branch prediction
#[inline(always)]
fn likely(condition: bool) -> bool {
    #[cold]
    fn cold() {}

    if !condition {
        cold();
    }
    condition
}

fn bench_branch_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Branch Prediction Hints");

    // Test with 90% true condition (like hot-path)
    let test_values: Vec<bool> = (0..100).map(|i| i < 90).collect();

    group.bench_function("Without likely() hint", |b| {
        b.iter(|| {
            let mut count = 0;
            for &val in &test_values {
                if black_box(val) {
                    count += 1;
                }
            }
            black_box(count)
        });
    });

    group.bench_function("With likely() hint", |b| {
        b.iter(|| {
            let mut count = 0;
            for &val in &test_values {
                if likely(black_box(val)) {
                    count += 1;
                }
            }
            black_box(count)
        });
    });

    group.finish();
}

// ========================================================================
// Combined Scenario Benchmarks
// ========================================================================

fn bench_realistic_event_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Realistic Event Parsing Scenarios");

    // Scenario 1: Small transaction (1-2 events, no strings) - Best case for SmallVec
    group.bench_function("Small tx (2 events, no strings)", |b| {
        b.iter(|| {
            let mut events: SmallVec<[u64; 4]> = SmallVec::new();
            events.push(black_box(100));
            events.push(black_box(200));
            // Simulate processing
            let sum: u64 = events.iter().sum();
            black_box(sum)
        });
    });

    // Scenario 2: Medium transaction (3-4 events) - Still on stack
    group.bench_function("Medium tx (4 events, no strings)", |b| {
        b.iter(|| {
            let mut events: SmallVec<[u64; 4]> = SmallVec::new();
            for i in 0..4 {
                events.push(black_box(i * 100));
            }
            let sum: u64 = events.iter().sum();
            black_box(sum)
        });
    });

    // Scenario 3: Large transaction (8 events) - Spills to heap
    group.bench_function("Large tx (8 events, no strings)", |b| {
        b.iter(|| {
            let mut events: SmallVec<[u64; 4]> = SmallVec::new();
            for i in 0..8 {
                events.push(black_box(i * 100));
            }
            let sum: u64 = events.iter().sum();
            black_box(sum)
        });
    });

    // Scenario 4: With string parsing (zero-copy)
    let string_data: Vec<u8> = vec![3, 0, 0, 0, b'S', b'O', b'L'];
    group.bench_function("Event with zero-copy string", |b| {
        b.iter(|| {
            let mut events: SmallVec<[u64; 4]> = SmallVec::new();
            events.push(black_box(100));

            // Zero-copy string parse
            let (token_ref, _) = read_string_ref(black_box(&string_data), 0).unwrap();
            let token_len = token_ref.len();

            events.push(black_box(token_len as u64));
            black_box(events)
        });
    });

    // Scenario 5: With string parsing (allocation)
    group.bench_function("Event with allocated string", |b| {
        b.iter(|| {
            let mut events: SmallVec<[u64; 4]> = SmallVec::new();
            events.push(black_box(100));

            // Allocated string parse
            let (token, _) = read_string(black_box(&string_data), 0).unwrap();
            let token_len = token.len();

            events.push(black_box(token_len as u64));
            black_box(events)
        });
    });

    group.finish();
}

// ========================================================================
// Criterion Configuration
// ========================================================================

criterion_group!(
    benches,
    bench_smallvec_stack_allocation,
    bench_zero_copy_strings,
    bench_text_field_extraction,
    bench_discriminator_lookup,
    bench_branch_prediction,
    bench_realistic_event_parsing
);

criterion_main!(benches);
