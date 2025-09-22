//! 高性能解析示例 - 展示函数式、零拷贝的解析能力
//!
//! 设计特点：
//! - 每个 DEX 独立的解析器
//! - 纯函数式设计
//! - 零拷贝优化
//! - 低延迟、高吞吐

use solana_streamer_sdk::{
    DexEvent, pumpfun_instruction, pumpfun_logs
};
use solana_sdk::{signature::Signature, pubkey::Pubkey};
use prost_types::Timestamp;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 高性能 Solana DEX 解析示例");
    println!("============================");

    // 演示1: PumpFun 指令解析
    demo_pumpfun_instruction_parsing().await?;

    // 演示2: PumpFun 日志解析
    demo_pumpfun_log_parsing().await?;

    // 演示3: 性能测试
    demo_performance_benchmark().await?;

    // 演示4: 批量处理
    demo_batch_processing().await?;

    Ok(())
}

/// 演示 PumpFun 指令解析
async fn demo_pumpfun_instruction_parsing() -> anyhow::Result<()> {
    println!("\n⚙️  演示1: PumpFun 指令解析");
    println!("---------------------------");

    // 模拟买入指令数据
    let mut instruction_data = vec![0u8; 32];
    instruction_data[..8].copy_from_slice(pumpfun_instruction::BUY_IX);
    instruction_data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // 1B tokens
    instruction_data[16..24].copy_from_slice(&500_000_000u64.to_le_bytes());  // 0.5 SOL

    // 模拟账户数组
    let accounts = vec![Pubkey::default(); 14];

    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(Timestamp { seconds: 1640995200, nanos: 0 });

    // 解析指令
    let start = Instant::now();
    let event = pumpfun_instruction::parse_pumpfun_instruction(
        &instruction_data,
        &accounts,
        signature,
        slot,
        block_time,
        0,
    );
    let parsing_time = start.elapsed();

    match event {
        Some(DexEvent::PumpFunTrade(trade)) => {
            println!("✅ 成功解析 PumpFun 交易事件:");
            println!("   交易类型: {}", if trade.is_buy { "买入" } else { "卖出" });
            println!("   代币数量: {}", trade.token_amount);
            println!("   SOL 数量: {}", trade.sol_amount);
            println!("   解析时间: {:?}", parsing_time);
        }
        Some(other) => {
            println!("✅ 解析到其他事件: {:?}", other);
        }
        None => {
            println!("❌ 未能解析事件");
        }
    }

    Ok(())
}

/// 演示 PumpFun 日志解析
async fn demo_pumpfun_log_parsing() -> anyhow::Result<()> {
    println!("\n📊 演示2: PumpFun 日志解析");
    println!("-------------------------");

    // 模拟交易日志
    let logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program log: Instruction: Buy".to_string(),
        "Program data: SGVsbG8gV29ybGQ=".to_string(), // 示例 base64 数据
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P consumed 50000 of 200000 compute units".to_string(),
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
    ];

    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(Timestamp { seconds: 1640995200, nanos: 0 });

    // 批量解析日志
    let start = Instant::now();
    let events = pumpfun_logs::parse_logs_batch(&logs, signature, slot, block_time.clone());
    let parsing_time = start.elapsed();

    println!("📈 批量解析结果:");
    println!("   发现 {} 个事件", events.len());
    println!("   解析时间: {:?}", parsing_time);

    // 流式解析日志（零分配）
    let start = Instant::now();
    let stream_events: Vec<_> = pumpfun_logs::parse_logs_stream(&logs, signature, slot, block_time)
        .collect();
    let stream_time = start.elapsed();

    println!("🌊 流式解析结果:");
    println!("   发现 {} 个事件", stream_events.len());
    println!("   解析时间: {:?}", stream_time);

    // 演示单个日志解析
    for log in &logs {
        if pumpfun_logs::is_pumpfun_log(log) {
            println!("✅ 识别到 PumpFun 日志: {}", &log[..50]);

            if let Some(data) = pumpfun_logs::extract_program_data(log) {
                println!("   提取到数据: {}", data);
            }
        }
    }

    Ok(())
}

/// 演示性能基准测试
async fn demo_performance_benchmark() -> anyhow::Result<()> {
    println!("\n⚡ 演示3: 性能基准测试");
    println!("---------------------");

    const ITERATIONS: usize = 10_000;

    // 准备测试数据
    let mut instruction_data = vec![0u8; 32];
    instruction_data[..8].copy_from_slice(pumpfun_instruction::BUY_IX);
    instruction_data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    instruction_data[16..24].copy_from_slice(&500_000_000u64.to_le_bytes());

    let accounts = vec![Pubkey::default(); 14];
    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(Timestamp { seconds: 1640995200, nanos: 0 });

    // 指令解析性能测试
    let start = Instant::now();
    let mut successful_parses = 0;

    for _ in 0..ITERATIONS {
        if let Some(_) = pumpfun_instruction::parse_pumpfun_instruction(
            &instruction_data,
            &accounts,
            signature,
            slot,
            block_time.clone(),
            0,
        ) {
            successful_parses += 1;
        }
    }

    let instruction_time = start.elapsed();
    let instruction_ops_per_sec = ITERATIONS as f64 / instruction_time.as_secs_f64();

    println!("🏎️  指令解析性能:");
    println!("   迭代次数: {}", ITERATIONS);
    println!("   成功解析: {}", successful_parses);
    println!("   总时间: {:?}", instruction_time);
    println!("   吞吐量: {:.0} ops/sec", instruction_ops_per_sec);
    println!("   平均延迟: {:?}", instruction_time / ITERATIONS as u32);

    // 程序ID检查性能测试
    let program_id = std::str::FromStr::from_str(pumpfun_instruction::PUMPFUN_PROGRAM_ID)?;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        let _ = pumpfun_instruction::is_pumpfun_program(&program_id);
    }

    let check_time = start.elapsed();
    let check_ops_per_sec = ITERATIONS as f64 / check_time.as_secs_f64();

    println!("\n🔍 程序ID检查性能:");
    println!("   吞吐量: {:.0} ops/sec", check_ops_per_sec);
    println!("   平均延迟: {:?}", check_time / ITERATIONS as u32);

    Ok(())
}

/// 演示批量处理
async fn demo_batch_processing() -> anyhow::Result<()> {
    println!("\n📦 演示4: 批量处理");
    println!("------------------");

    // 生成测试数据
    let batch_size = 1000;
    let mut instructions = Vec::with_capacity(batch_size);

    for i in 0..batch_size {
        let mut data = vec![0u8; 32];
        data[..8].copy_from_slice(pumpfun_instruction::BUY_IX);
        data[8..16].copy_from_slice(&(1_000_000_000u64 + i as u64).to_le_bytes());
        data[16..24].copy_from_slice(&(500_000_000u64 + i as u64).to_le_bytes());

        let accounts = vec![Pubkey::default(); 14];
        instructions.push((data, accounts));
    }

    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(Timestamp { seconds: 1640995200, nanos: 0 });

    // 批量解析
    let start = Instant::now();
    let events = pumpfun_instruction::parse_instructions_batch(
        &instructions,
        signature,
        slot,
        block_time,
    );
    let batch_time = start.elapsed();

    println!("📊 批量处理结果:");
    println!("   输入指令数: {}", batch_size);
    println!("   解析事件数: {}", events.len());
    println!("   处理时间: {:?}", batch_time);
    println!("   批量吞吐量: {:.0} instructions/sec",
             batch_size as f64 / batch_time.as_secs_f64());

    // 验证事件类型
    let trade_events = events.iter()
        .filter(|e| matches!(e, DexEvent::PumpFunTrade(_)))
        .count();

    println!("   交易事件数: {}", trade_events);

    Ok(())
}

/// 展示内存使用和零拷贝特性
#[allow(dead_code)]
fn demo_zero_copy_features() {
    println!("\n🧠 零拷贝特性:");
    println!("---------------");
    println!("✅ 指令判别符匹配 - 编译时常量比较");
    println!("✅ 字符串解析 - unsafe 零拷贝转换");
    println!("✅ 数值解析 - 直接内存访问");
    println!("✅ 批量处理 - 预分配容量");
    println!("✅ 流式处理 - 惰性迭代器");
}

/// 展示性能优化技术
#[allow(dead_code)]
fn demo_performance_optimizations() {
    println!("\n⚡ 性能优化技术:");
    println!("-----------------");
    println!("🚀 SIMD 判别符匹配");
    println!("🚀 分支预测优化");
    println!("🚀 内联函数标注");
    println!("🚀 编译时常量计算");
    println!("🚀 零动态分配");
    println!("🚀 向量化批处理");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_examples() {
        let result = demo_pumpfun_instruction_parsing().await;
        assert!(result.is_ok());

        let result = demo_pumpfun_log_parsing().await;
        assert!(result.is_ok());

        let result = demo_performance_benchmark().await;
        assert!(result.is_ok());

        let result = demo_batch_processing().await;
        assert!(result.is_ok());
    }
}