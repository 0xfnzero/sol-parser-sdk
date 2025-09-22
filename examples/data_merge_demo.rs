//! 数据合并演示 - 展示指令和日志数据智能合并的效果
//!
//! 这个示例展示：
//! 1. 指令数据包含用户意图和账户信息
//! 2. 日志数据包含真实执行结果
//! 3. 智能合并后获得最完整的事件数据

use solana_streamer_sdk::parser::pumpfun_ix_parser;
use solana_sdk::{signature::Signature, pubkey::Pubkey};

fn main() -> anyhow::Result<()> {
    println!("🔗 指令+日志数据智能合并演示");
    println!("==============================");

    // 模拟一个 PumpFun 买入交易
    demo_pumpfun_data_merge()?;

    Ok(())
}

fn demo_pumpfun_data_merge() -> anyhow::Result<()> {
    println!("\n📊 PumpFun 买入交易数据合并");
    println!("---------------------------");

    // 1. 创建指令数据（用户意图）
    let mut instruction_data = vec![0u8; 32];
    // PumpFun 买入指令判别符
    instruction_data[..8].copy_from_slice(&[102, 6, 61, 18, 1, 218, 235, 234]);
    // 用户想买 1B 代币
    instruction_data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    // 用户最多愿意花费 0.5 SOL
    instruction_data[16..24].copy_from_slice(&500_000_000u64.to_le_bytes());

    // 2. 创建账户信息（指令中包含）
    let accounts = vec![
        Pubkey::new_unique(), // global
        Pubkey::new_unique(), // fee_recipient
        Pubkey::new_unique(), // mint
        Pubkey::new_unique(), // bonding_curve
        Pubkey::new_unique(), // associated_bonding_curve
        Pubkey::new_unique(), // associated_user
        Pubkey::new_unique(), // user
        Pubkey::new_unique(), // system_program
        Pubkey::new_unique(), // token_program
        Pubkey::new_unique(), // creator_vault
        Pubkey::new_unique(), // event_authority
        Pubkey::new_unique(), // program
        Pubkey::new_unique(), // global_volume_accumulator
        Pubkey::new_unique(), // user_volume_accumulator
    ];

    // 3. 创建日志数据（真实执行结果）
    let logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program log: Instruction: Buy".to_string(),
        // 这里包含 base64 编码的交易日志，包含实际执行数据：
        // - 实际花费: 0.48 SOL (480000000 lamports)
        // - 实际获得: 950000000 代币
        // - 虚拟储备: virtual_sol=100000000000, virtual_token=50000000000000
        // - 真实储备: real_sol=50000000000, real_token=25000000000000
        // - 费用: 24000000 (5%)
        "Program data: SGVsbG8gV29ybGQ=".to_string(), // 示例数据
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P consumed 45000 of 200000 compute units".to_string(),
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
    ];

    // 4. 使用智能合并解析器
    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(1640995200000);
    let instruction_index = 0;

    if let Some(merged_event) = pumpfun_ix_parser::parse_pumpfun_transaction(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        instruction_index,
    ) {
        println!("✅ 成功合并指令和日志数据！");

        if let solana_streamer_sdk::DexEvent::PumpFunTrade(trade) = merged_event {
            println!("\n📋 合并后的完整数据：");

            println!("🎯 用户意图 (从指令):");
            println!("  - 期望代币数量: {} tokens", trade.amount);
            println!("  - 最大 SOL 成本: {} SOL", trade.max_sol_cost as f64 / 1e9);
            println!("  - 最小 SOL 输出: {} SOL", trade.min_sol_output as f64 / 1e9);

            println!("\n💰 实际执行结果 (从日志):");
            println!("  - 实际 SOL 金额: {} SOL", trade.sol_amount as f64 / 1e9);
            println!("  - 实际代币数量: {} tokens", trade.token_amount);
            println!("  - 虚拟 SOL 储备: {} SOL", trade.virtual_sol_reserves as f64 / 1e9);
            println!("  - 虚拟代币储备: {} tokens", trade.virtual_token_reserves);
            println!("  - 实际费用: {} SOL", trade.fee as f64 / 1e9);
            println!("  - 创作者费用: {} SOL", trade.creator_fee as f64 / 1e9);

            println!("\n🏛️ 账户信息 (从指令):");
            println!("  - 代币地址: {}", trade.mint);
            println!("  - 绑定曲线: {}", trade.bonding_curve);
            println!("  - 用户地址: {}", trade.user);

            println!("\n🤖 智能分析:");
            println!("  - 是否机器人: {}", trade.is_bot);
            println!("  - 交易类型: {}", if trade.is_buy { "买入" } else { "卖出" });

            let efficiency = if trade.max_sol_cost > 0 {
                (trade.sol_amount as f64 / trade.max_sol_cost as f64) * 100.0
            } else {
                0.0
            };
            println!("  - 成本效率: {:.1}%", efficiency);
        }
    } else {
        println!("❌ 解析失败");
    }

    println!("\n💡 合并优势：");
    println!("  ✅ 从指令获取用户意图和完整账户信息");
    println!("  ✅ 从日志获取真实执行结果和链上状态");
    println!("  ✅ 智能检测机器人行为和异常交易");
    println!("  ✅ 提供最完整的事件数据用于分析");

    Ok(())
}