use sol_parser_sdk::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

fn main() {
    println!("🧪 测试预解析账户数据逻辑");

    // 模拟真实的指令和账户数据
    let instruction_data = vec![1, 2, 3, 4]; // 指令数据
    let accounts = vec![
        Pubkey::new_unique(), // 用户账户
        Pubkey::new_unique(), // 代币mint
        Pubkey::new_unique(), // 池子地址
        Pubkey::new_unique(), // AMM地址
        Pubkey::new_unique(), // Whirlpool地址
    ];

    // 模拟多个不同协议的日志事件
    let logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program data: aGVsbG8gd29ybGQ=".to_string(), // PumpFun交易日志
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),

        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 invoke [1]".to_string(),
        "Program log: Raydium CLMM swap executed".to_string(), // Raydium CLMM日志
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 success".to_string(),

        "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]".to_string(),
        "Program data: bXVsdGkgZGF0YQ==".to_string(), // Orca Whirlpool日志
        "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc success".to_string(),
    ];

    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(1640995200);
    let program_id = Pubkey::default();

    println!("\n📋 测试场景:");
    println!("  - 指令数据: {} bytes", instruction_data.len());
    println!("  - 账户数量: {}", accounts.len());
    println!("  - 日志行数: {}", logs.len());

    println!("\n🔍 预定义账户地址:");
    for (i, account) in accounts.iter().enumerate() {
        let label = match i {
            0 => "用户账户",
            1 => "代币Mint",
            2 => "池子地址",
            3 => "AMM地址",
            4 => "Whirlpool地址",
            _ => "其他账户",
        };
        println!("  {}[{}]: {}", label, i, account);
    }

    println!("\n🔍 日志内容:");
    for (i, log) in logs.iter().enumerate() {
        println!("  日志{}: {}", i + 1, log);
    }

    let mut callback_count = 0;
    let mut callback_events = Vec::new();

    println!("\n⚡ 开始预解析逻辑 (先解析指令→再逐个处理log事件并补充账户):");
    parse_transaction_events_streaming(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        &program_id,
        |event| {
            callback_count += 1;
            println!("📤 立即回调事件 {}: {:?}", callback_count, get_event_brief(&event));

            // 检查账户地址是否被正确填充
            print_detailed_account_info(&event, callback_count, &accounts);

            callback_events.push(event);
        }
    );

    println!("\n✅ 预解析逻辑完成统计:");
    println!("  - 回调次数: {}", callback_count);
    println!("  - 事件总数: {}", callback_events.len());

    if callback_events.is_empty() {
        println!("⚠️  未解析出任何事件 (这在测试数据下是正常的)");
        println!("    💡 真实环境中，指令数据会包含实际的交易信息");
    } else {
        println!("🎯 解析出的事件类型:");
        for (i, event) in callback_events.iter().enumerate() {
            println!("  {}. {}", i + 1, get_event_brief(event));
        }

        println!("\n📊 账户填充效果分析:");
        analyze_account_filling(&callback_events, &accounts);
    }

    println!("\n🔧 新的预解析逻辑特性:");
    println!("  ✅ 指令预解析 - 开始前一次性解析指令获取所有账户数据");
    println!("  ✅ 结构化存储 - 使用InstructionAccountData结构体存储账户信息");
    println!("  ✅ 高效填充 - log事件解析后直接从预解析数据中补充账户");
    println!("  ✅ 数值补充 - 不仅填充账户地址，还补充金额等数值数据");
    println!("  ✅ 类型安全 - 只在相同事件类型间进行数据补充");
    println!("  ✅ 避免重复 - 一次指令解析，多次复用账户数据");
}

fn get_event_brief(event: &DexEvent) -> &'static str {
    match event {
        DexEvent::PumpFunTrade(_) => "PumpFun交易",
        DexEvent::PumpFunCreate(_) => "PumpFun创建",
        DexEvent::BonkTrade(_) => "Bonk交易",
        DexEvent::RaydiumClmmSwap(_) => "Raydium CLMM交换",
        DexEvent::RaydiumCpmmSwap(_) => "Raydium CPMM交换",
        DexEvent::RaydiumAmmV4Swap(_) => "Raydium AMM V4交换",
        DexEvent::OrcaWhirlpoolSwap(_) => "Orca Whirlpool交换",
        DexEvent::MeteoraPoolsSwap(_) => "Meteora Pools交换",
        DexEvent::MeteoraDammV2Swap(_) => "Meteora DAMM V2交换",
        _ => "其他事件",
    }
}

fn print_detailed_account_info(event: &DexEvent, event_num: usize, expected_accounts: &[Pubkey]) {
    println!("    📍 事件{}详细账户信息:", event_num);

    match event {
        DexEvent::PumpFunTrade(trade) => {
            println!("       PumpFun交易:");
            println!("         用户: {} {}", trade.user,
                if trade.user != Pubkey::default() { "✅已填充" } else { "❌默认值" });
            println!("         代币: {} {}", trade.mint,
                if trade.mint != Pubkey::default() { "✅已填充" } else { "❌默认值" });
            println!("         SOL数量: {}", trade.sol_amount);
            println!("         代币数量: {}", trade.token_amount);
        },
        DexEvent::RaydiumClmmSwap(swap) => {
            println!("       Raydium CLMM交换:");
            println!("         池子: {} {}", swap.pool,
                if swap.pool != Pubkey::default() { "✅已填充" } else { "❌默认值" });
            println!("         用户: {} {}", swap.user,
                if swap.user != Pubkey::default() { "✅已填充" } else { "❌默认值" });
            println!("         数量: {}", swap.amount);
        },
        DexEvent::OrcaWhirlpoolSwap(swap) => {
            println!("       Orca Whirlpool交换:");
            println!("         池子: {} {}", swap.whirlpool,
                if swap.whirlpool != Pubkey::default() { "✅已填充" } else { "❌默认值" });
            println!("         输入数量: {}", swap.input_amount);
            println!("         输出数量: {}", swap.output_amount);
        },
        _ => {
            println!("       其他类型事件");
        }
    }
}

fn analyze_account_filling(events: &[DexEvent], expected_accounts: &[Pubkey]) {
    let mut filled_accounts = 0;
    let mut total_account_fields = 0;

    for event in events {
        match event {
            DexEvent::PumpFunTrade(trade) => {
                total_account_fields += 2;
                if trade.user != Pubkey::default() { filled_accounts += 1; }
                if trade.mint != Pubkey::default() { filled_accounts += 1; }
            },
            DexEvent::RaydiumClmmSwap(swap) => {
                total_account_fields += 2;
                if swap.pool != Pubkey::default() { filled_accounts += 1; }
                if swap.user != Pubkey::default() { filled_accounts += 1; }
            },
            DexEvent::OrcaWhirlpoolSwap(swap) => {
                total_account_fields += 1;
                if swap.whirlpool != Pubkey::default() { filled_accounts += 1; }
            },
            _ => {}
        }
    }

    if total_account_fields > 0 {
        let fill_rate = (filled_accounts as f64 / total_account_fields as f64) * 100.0;
        println!("  📈 账户填充率: {:.1}% ({}/{})", fill_rate, filled_accounts, total_account_fields);

        if fill_rate >= 80.0 {
            println!("  🎉 账户填充效果很好！");
        } else if fill_rate >= 50.0 {
            println!("  👍 账户填充效果良好");
        } else {
            println!("  ⚠️  账户填充率较低，可能需要优化指令解析");
        }
    } else {
        println!("  📝 没有检测到需要填充的账户字段");
    }
}