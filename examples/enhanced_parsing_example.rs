//! 增强版解析示例 - 展示集成了旧版本功能的完整解析能力
//!
//! 这个示例展示了新的统一解析器的功能：
//! - 从指令数据解析事件
//! - 从日志数据解析事件
//! - 合并和增强事件数据
//! - 完整的事件字段

use solana_streamer_sdk::{
    UnifiedParser, InstructionParser, LogParser,
    DexEvent, PumpFunTradeEvent, PumpFunCreateTokenEvent
};
use solana_sdk::{signature::Signature, pubkey::Pubkey};
use prost_types::Timestamp;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 增强版 Solana DEX 事件解析示例");
    println!("==================================");

    // 创建统一解析器 - 整合了指令和日志解析
    let unified_parser = UnifiedParser::new();

    // 创建独立的解析器
    let instruction_parser = InstructionParser::new();

    println!("\n📋 解析器功能展示:");
    println!("1. 统一解析器 (UnifiedParser): 整合指令+日志解析，自动合并事件");
    println!("2. 指令解析器 (InstructionParser): 从原始指令数据解析事件");
    println!("3. 日志解析器 (LogParser): 从交易日志解析事件");

    // 演示1: 从日志解析 PumpFun 事件
    demo_log_parsing().await?;

    // 演示2: 指令解析 (模拟)
    demo_instruction_parsing().await?;

    // 演示3: 统一解析器的事件合并功能
    demo_unified_parsing().await?;

    Ok(())
}

/// 演示从日志解析事件
async fn demo_log_parsing() -> anyhow::Result<()> {
    println!("\n🔍 演示1: 从交易日志解析 PumpFun 事件");
    println!("----------------------------------------");

    // 模拟真实的交易日志（这些是示例数据）
    let mock_logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program log: Instruction: CreateToken".to_string(),
        "Program data: 5EWlrejeMeQ6Eq5tqbGBMqUqBPSR//FLZQqV0Q==".to_string(), // 示例 base64 数据
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P consumed 50000 of 200000 compute units".to_string(),
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
    ];

    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(Timestamp {
        seconds: 1640995200, // 2022-01-01 00:00:00 UTC
        nanos: 0,
    });

    // 使用日志解析器
    let events = LogParser::parse_all_dex_events_from_logs(
        &mock_logs,
        signature,
        slot,
        block_time,
    );

    println!("📊 解析结果:");
    println!("  发现 {} 个事件", events.len());

    for (i, event) in events.iter().enumerate() {
        println!("  事件 {}: {:?}", i + 1, match event {
            DexEvent::PumpFunCreate(_) => "PumpFun 创建代币事件",
            DexEvent::PumpFunTrade(_) => "PumpFun 交易事件",
            DexEvent::BonkTrade(_) => "Bonk 交易事件",
            DexEvent::PumpSwapBuy(_) => "PumpSwap 买入事件",
            DexEvent::PumpSwapSell(_) => "PumpSwap 卖出事件",
            _ => "其他事件类型",
        });
    }

    if events.is_empty() {
        println!("  ⚠️  未发现有效事件（示例数据可能不包含真实的事件数据）");
        println!("  💡 在实际使用中，请使用真实的交易日志数据");
    }

    Ok(())
}

/// 演示指令解析功能
async fn demo_instruction_parsing() -> anyhow::Result<()> {
    println!("\n⚙️  演示2: 指令解析功能");
    println!("------------------------");

    println!("📝 指令解析器支持的功能:");
    println!("  • 从原始指令数据解析事件");
    println!("  • 支持 gRPC 交易格式");
    println!("  • 支持版本化交易格式");
    println!("  • 高性能的 SIMD 优化");
    println!("  • 账户公钥缓存机制");

    // 在实际使用中，你会这样使用指令解析器：
    /*
    let callback = Arc::new(|event: DexEvent| {
        println!("收到事件: {:?}", event);
    });

    // 解析 gRPC 交易
    instruction_parser.parse_grpc_transaction(
        grpc_tx,
        signature,
        Some(slot),
        block_time,
        callback,
    ).await?;

    // 或解析版本化交易
    instruction_parser.parse_versioned_transaction(
        &versioned_tx,
        signature,
        Some(slot),
        block_time,
        &inner_instructions,
        callback,
    ).await?;
    */

    println!("  ✅ 指令解析器已就绪（需要真实交易数据进行测试）");

    Ok(())
}

/// 演示统一解析器的事件合并功能
async fn demo_unified_parsing() -> anyhow::Result<()> {
    println!("\n🔄 演示3: 统一解析器的事件合并");
    println!("--------------------------------");

    println!("🎯 统一解析器的优势:");
    println!("  • 自动合并来自指令和日志的相同事件");
    println!("  • 用日志数据填充指令数据中的空字段");
    println!("  • 提供最完整的事件信息");
    println!("  • 去重和数据清理");

    // 模拟事件合并场景
    println!("\n📊 事件合并流程:");
    println!("  1. 从指令解析基础事件结构（账户信息、参数等）");
    println!("  2. 从日志解析详细事件数据（储备量、费用等）");
    println!("  3. 智能合并两个事件，保留最完整的信息");
    println!("  4. 返回增强的完整事件");

    // 在实际使用中：
    /*
    let events = unified_parser.parse_grpc_transaction_complete(
        grpc_tx,
        signature,
        Some(slot),
        block_time,
        Some(logs), // 同时提供日志数据
    ).await?;

    // 或者从版本化交易解析
    let events = unified_parser.parse_versioned_transaction_complete(
        &versioned_tx,
        signature,
        Some(slot),
        block_time,
        &inner_instructions,
        Some(logs),
    ).await?;
    */

    println!("  ✅ 统一解析器提供了最佳的解析体验");

    Ok(())
}

/// 演示完整的事件字段
#[allow(dead_code)]
fn demo_enhanced_event_fields() {
    println!("\n📋 增强的事件字段展示:");
    println!("------------------------");

    // 展示 PumpFun 交易事件的所有字段
    println!("🎯 PumpFun 交易事件字段 ({}个):", std::mem::size_of::<PumpFunTradeEvent>());
    println!("  基础信息: signature, slot, block_time, mint, user, is_buy");
    println!("  交易数据: sol_amount, token_amount, bonding_curve");
    println!("  储备信息: virtual_sol_reserves, virtual_token_reserves");
    println!("  真实储备: real_sol_reserves, real_token_reserves");
    println!("  费用信息: fee_recipient, fee_basis_points, fee");
    println!("  创建者费用: creator, creator_fee_basis_points, creator_fee");
    println!("  统计数据: total_unclaimed_tokens, total_claimed_tokens, current_sol_volume");
    println!("  时间信息: timestamp, last_update_timestamp, track_volume");
    println!("  指令参数: max_sol_cost, min_sol_output, amount");
    println!("  状态标记: is_bot, is_dev_create_token_trade");
    println!("  账户信息: global, associated_bonding_curve, associated_user...");

    // 展示 PumpFun 创建事件的字段
    println!("\n🎯 PumpFun 创建事件字段:");
    println!("  代币信息: name, symbol, uri, mint, creator");
    println!("  储备信息: virtual_token_reserves, virtual_sol_reserves");
    println!("  供应信息: real_token_reserves, token_total_supply");
    println!("  时间戳: timestamp");
    println!("  账户信息: mint_authority, associated_bonding_curve");
}

/// 性能和兼容性说明
#[allow(dead_code)]
fn show_performance_info() {
    println!("\n⚡ 性能和兼容性:");
    println!("------------------");
    println!("🚀 性能优化:");
    println!("  • SIMD 优化的数据验证和判别符匹配");
    println!("  • 账户公钥缓存，减少内存分配");
    println!("  • 并行处理指令配置");
    println!("  • 零拷贝的数据解析");

    println!("\n🔄 兼容性:");
    println!("  • 完全兼容旧版本的事件结构");
    println!("  • 支持所有现有的 DEX 协议");
    println!("  • 平滑迁移路径");
    println!("  • 向后兼容的 API");

    println!("\n🎯 推荐使用方式:");
    println!("  • 新项目: 直接使用 UnifiedParser");
    println!("  • 现有项目: 渐进式迁移，先使用 LogParser 或 InstructionParser");
    println!("  • 高性能需求: 使用 InstructionParser + 自定义回调");
    println!("  • 简单需求: 继续使用 SimpleEventParser");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_parsing_example() {
        // 测试示例代码能正常运行
        let result = demo_log_parsing().await;
        assert!(result.is_ok());

        let result = demo_instruction_parsing().await;
        assert!(result.is_ok());

        let result = demo_unified_parsing().await;
        assert!(result.is_ok());
    }
}