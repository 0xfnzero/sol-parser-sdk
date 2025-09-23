use std::time::Instant;
use sol_parser_sdk::{DexEvent, parse_transaction_events_streaming, parse_logs_streaming};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 性能测试：对比优化前后的解析速度
fn main() {
    println!("🚀 Sol Parser SDK 解析性能测试");

    // 模拟真实的日志数据
    let test_logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program data: aGVsbG8gd29ybGQ=".to_string(),
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
        "Program log: Trade executed".to_string(),
        "Program data: dGVzdCBkYXRh".to_string(),
        "Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 invoke [1]".to_string(),
        "Program log: Raydium AMM V4 swap".to_string(),
        "Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 success".to_string(),
        "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]".to_string(),
        "Program data: bXVsdGkgZGF0YQ==".to_string(),
    ];

    // 测试参数
    let iterations = 10_000;
    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(1640995200);

    println!("\n📊 日志解析性能测试 ({} 次迭代):", iterations);

    // 测试优化后的日志解析器
    println!("\n--- 优化后的日志解析器 ---");
    let start = Instant::now();
    let mut optimized_events = 0;

    for _ in 0..iterations {
        for log in &test_logs {
            if let Some(_event) = sol_parser_sdk::logs::parse_log_unified(log, signature, slot, block_time) {
                optimized_events += 1;
            }
        }
    }

    let optimized_duration = start.elapsed();
    let optimized_time_per_log = optimized_duration.as_nanos() / (iterations as u128 * test_logs.len() as u128);

    println!("总耗时: {:?}", optimized_duration);
    println!("平均每个日志: {}ns", optimized_time_per_log);
    println!("解析出的事件: {}", optimized_events);

    // 测试传统日志解析器
    println!("\n--- 传统日志解析器 ---");
    let start = Instant::now();
    let mut legacy_events = 0;

    for _ in 0..iterations {
        for log in &test_logs {
            if let Some(_event) = sol_parser_sdk::logs::parse_log_unified_legacy(log, signature, slot, block_time) {
                legacy_events += 1;
            }
        }
    }

    let legacy_duration = start.elapsed();
    let legacy_time_per_log = legacy_duration.as_nanos() / (iterations as u128 * test_logs.len() as u128);

    println!("总耗时: {:?}", legacy_duration);
    println!("平均每个日志: {}ns", legacy_time_per_log);
    println!("解析出的事件: {}", legacy_events);

    // 计算性能提升
    let improvement_ratio = legacy_time_per_log as f64 / optimized_time_per_log as f64;

    println!("\n🎯 性能提升结果:");
    println!("优化后速度提升: {:.2}x", improvement_ratio);
    println!("单个日志解析节省: {}ns", legacy_time_per_log - optimized_time_per_log);

    // 流式解析性能测试
    println!("\n📊 流式解析性能测试:");

    let instruction_data = vec![1, 2, 3, 4];
    let accounts = vec![Pubkey::default(); 5];
    let program_id = Pubkey::default();

    // 测试流式解析性能
    println!("\n--- 流式解析测试 ---");
    let start = Instant::now();
    let mut stream_events = 0;

    for _ in 0..(iterations / 100) { // 减少迭代次数因为这是更复杂的测试
        parse_transaction_events_streaming(
            &instruction_data,
            &accounts,
            &test_logs,
            signature,
            slot,
            block_time,
            &program_id,
            |_event| {
                stream_events += 1;
            }
        );
    }

    let stream_duration = start.elapsed();
    let stream_time_per_transaction = stream_duration.as_micros() / (iterations as u128 / 100);

    println!("总耗时: {:?}", stream_duration);
    println!("平均每个交易: {}μs", stream_time_per_transaction);
    println!("流式处理的事件: {}", stream_events);

    // 内存使用情况测试
    println!("\n💾 内存使用情况:");

    // 测试大量解析的内存效率
    let large_test_iterations = 100_000;
    let start = Instant::now();

    for _ in 0..large_test_iterations {
        for log in &test_logs {
            let _ = sol_parser_sdk::logs::optimized_matcher::detect_log_type(log);
        }
    }

    let detection_duration = start.elapsed();
    let detection_time_per_log = detection_duration.as_nanos() / (large_test_iterations as u128 * test_logs.len() as u128);

    println!("日志类型检测 ({} 次):", large_test_iterations);
    println!("总耗时: {:?}", detection_duration);
    println!("平均每个日志: {}ns", detection_time_per_log);

    println!("\n✅ 性能测试完成!");
    println!("主要优化:");
    println!("  🔧 预计算程序ID字符串，避免format!()调用");
    println!("  ⚡ 优化日志类型检测，减少字符串搜索");
    println!("  🚀 预分配向量容量，减少内存重新分配");
    println!("  ⏰ 减少重复的时间戳计算");
    println!("  📊 按使用频率排序协议检查顺序");
}

/// 单独的微基准测试
#[cfg(test)]
mod micro_benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_string_operations() {
        let program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
        let iterations = 1_000_000;

        // 测试 format! vs 预计算字符串
        let start = Instant::now();
        for _ in 0..iterations {
            let _s = format!("Program {} invoke", program_id);
        }
        let format_time = start.elapsed();

        let precomputed = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke";
        let start = Instant::now();
        for _ in 0..iterations {
            let _s = precomputed;
        }
        let precomputed_time = start.elapsed();

        println!("format!() time: {:?}", format_time);
        println!("precomputed time: {:?}", precomputed_time);
        println!("improvement: {:.2}x", format_time.as_nanos() as f64 / precomputed_time.as_nanos() as f64);
    }

    #[test]
    fn test_vector_allocation() {
        let iterations = 100_000;
        let data = vec![1u8; 32];

        // 测试 filter_map + collect vs 预分配
        let start = Instant::now();
        for _ in 0..iterations {
            let _accounts: Vec<String> = data.iter()
                .filter_map(|&x| if x > 0 { Some(x.to_string()) } else { None })
                .collect();
        }
        let filter_map_time = start.elapsed();

        let start = Instant::now();
        for _ in 0..iterations {
            let mut accounts = Vec::with_capacity(data.len());
            for &x in &data {
                if x > 0 {
                    accounts.push(x.to_string());
                }
            }
        }
        let preallocated_time = start.elapsed();

        println!("filter_map time: {:?}", filter_map_time);
        println!("preallocated time: {:?}", preallocated_time);
        println!("improvement: {:.2}x", filter_map_time.as_nanos() as f64 / preallocated_time.as_nanos() as f64);
    }
}