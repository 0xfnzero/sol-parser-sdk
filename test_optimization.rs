#!/usr/bin/env rust-script

//! Simple test script to verify the optimization works correctly
//! Run with: cargo script test_optimization.rs

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::time::Instant;

// Original inefficient approach (using string conversion)
fn old_approach(program_id: &Pubkey, target_program_id: &str) -> bool {
    program_id.to_string() == target_program_id
}

// New optimized approach (direct Pubkey comparison)
fn new_approach(program_id: &Pubkey, target_program_id_pubkey: &Pubkey) -> bool {
    *program_id == *target_program_id_pubkey
}

fn main() {
    // Test constants
    const PUMPFUN_PROGRAM_ID_STR: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
    const PUMPFUN_PROGRAM_ID_PUBKEY: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

    let test_program_id = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
    let non_matching_id = pubkey!("11111111111111111111111111111111");

    // Correctness test
    println!("=== Correctness Test ===");

    // Test matching case
    let old_result_match = old_approach(&test_program_id, PUMPFUN_PROGRAM_ID_STR);
    let new_result_match = new_approach(&test_program_id, &PUMPFUN_PROGRAM_ID_PUBKEY);
    println!("Matching case - Old: {}, New: {}", old_result_match, new_result_match);
    assert_eq!(old_result_match, new_result_match);
    assert!(old_result_match && new_result_match);

    // Test non-matching case
    let old_result_no_match = old_approach(&non_matching_id, PUMPFUN_PROGRAM_ID_STR);
    let new_result_no_match = new_approach(&non_matching_id, &PUMPFUN_PROGRAM_ID_PUBKEY);
    println!("Non-matching case - Old: {}, New: {}", old_result_no_match, new_result_no_match);
    assert_eq!(old_result_no_match, new_result_no_match);
    assert!(!old_result_no_match && !new_result_no_match);

    // Performance test
    println!("\n=== Performance Test ===");
    const ITERATIONS: usize = 1_000_000;

    // Test old approach
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = old_approach(&test_program_id, PUMPFUN_PROGRAM_ID_STR);
    }
    let old_duration = start.elapsed();

    // Test new approach
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = new_approach(&test_program_id, &PUMPFUN_PROGRAM_ID_PUBKEY);
    }
    let new_duration = start.elapsed();

    println!("Old approach (string conversion): {:?}", old_duration);
    println!("New approach (direct comparison): {:?}", new_duration);

    let speedup = old_duration.as_nanos() as f64 / new_duration.as_nanos() as f64;
    println!("Speedup: {:.2}x faster", speedup);

    if speedup > 1.0 {
        println!("✅ Optimization successful!");
    } else {
        println!("❌ Optimization may not be effective");
    }
}