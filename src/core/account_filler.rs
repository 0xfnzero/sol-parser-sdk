//! 账户填充模块
//!
//! 负责从指令账户数据填充DEX事件中缺失的账户字段
//! 每个平台的每个事件类型都有专门的填充函数
//! 只填充那些会变化的账户，排除系统程序等常量账户

use solana_sdk::pubkey::Pubkey;
use crate::core::events::*;

/// 账户获取辅助函数类型
type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 主要的账户填充调度函数
pub fn fill_accounts_from_instruction_data(
    event: &mut DexEvent,
    instruction_accounts: &[Pubkey],
) {
    // 获取账户的辅助函数
    let get_account = |index: usize| -> Pubkey {
        instruction_accounts.get(index).cloned().unwrap_or_default()
    };

    match event {
        // PumpFun 事件填充
        DexEvent::PumpFunTrade(ref mut trade_event) => {
            pumpfun::fill_trade_accounts(trade_event, &get_account);
        },
        DexEvent::PumpFunCreate(ref mut create_event) => {
            pumpfun::fill_create_accounts(create_event, &get_account);
        },
        DexEvent::PumpFunMigrate(ref mut migrate_event) => {
            pumpfun::fill_migrate_accounts(migrate_event, &get_account);
        },

        // Raydium 事件填充
        DexEvent::RaydiumClmmSwap(ref mut swap_event) => {
            raydium::fill_clmm_swap_accounts(swap_event, &get_account);
        },
        DexEvent::RaydiumCpmmSwap(ref mut swap_event) => {
            raydium::fill_cpmm_swap_accounts(swap_event, &get_account);
        },
        DexEvent::RaydiumAmmV4Swap(ref mut swap_event) => {
            raydium::fill_amm_v4_swap_accounts(swap_event, &get_account);
        },

        // Orca 事件填充
        DexEvent::OrcaWhirlpoolSwap(ref mut swap_event) => {
            orca::fill_whirlpool_swap_accounts(swap_event, &get_account);
        },

        // Meteora 事件填充
        DexEvent::MeteoraPoolsSwap(ref mut swap_event) => {
            meteora::fill_pools_swap_accounts(swap_event, &get_account);
        },
        DexEvent::MeteoraDammV2Swap(ref mut swap_event) => {
            meteora::fill_damm_v2_swap_accounts(swap_event, &get_account);
        },

        // Bonk 事件填充
        DexEvent::BonkTrade(ref mut trade_event) => {
            bonk::fill_trade_accounts(trade_event, &get_account);
        },

        // 其他事件类型暂时不处理
        _ => {}
    }
}

/// PumpFun 账户填充模块
pub mod pumpfun {
    use super::*;

    /// 填充 PumpFun Trade 事件账户
    /// 基于PumpFun IDL的buy/sell指令账户映射
    pub fn fill_trade_accounts(
        trade_event: &mut PumpFunTradeEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // PumpFun buy/sell指令的共同账户映射 (基于IDL):
        // 0: global - 全局配置
        // 1: fee_recipient - 费用接收方
        // 2: mint - 代币mint
        // 3: bonding_curve - 绑定曲线
        // 4: associated_bonding_curve - 关联绑定曲线
        // 5: associated_user - 关联用户账户
        // 6: user - 用户账户

        // 基于最新IDL buy/sell指令账户映射:
        // 0: global - 全局配置PDA
        // 1: fee_recipient - 费用接收方 (现在是事件字段，不再从账户填充)
        // 2: mint - 代币mint
        // 3: bonding_curve - 绑定曲线PDA
        // 4: associated_bonding_curve - 关联绑定曲线PDA
        // 5: associated_user - 关联用户PDA
        // 6: user - 用户账户

        if trade_event.global == Pubkey::default() {
            trade_event.global = get_account(0);
        }
        if trade_event.mint == Pubkey::default() {
            trade_event.mint = get_account(2);
        }
        if trade_event.bonding_curve == Pubkey::default() {
            trade_event.bonding_curve = get_account(3);
        }
        if trade_event.associated_bonding_curve == Pubkey::default() {
            trade_event.associated_bonding_curve = get_account(4);
        }
        if trade_event.associated_user == Pubkey::default() {
            trade_event.associated_user = get_account(5);
        }
        if trade_event.user == Pubkey::default() {
            trade_event.user = get_account(6);
        }
    }

    /// 填充 PumpFun Create 事件账户
    pub fn fill_create_accounts(
        create_event: &mut PumpFunCreateTokenEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // TODO: 基于PumpFun create指令IDL定义账户映射
        if create_event.mint == Pubkey::default() {
            create_event.mint = get_account(0);
        }
        if create_event.user == Pubkey::default() {
            create_event.user = get_account(7); // 基于IDL create指令
        }
        if create_event.bonding_curve == Pubkey::default() {
            create_event.bonding_curve = get_account(2);
        }
    }

    /// 填充 PumpFun Migrate 事件账户
    pub fn fill_migrate_accounts(
        migrate_event: &mut PumpFunMigrateEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // TODO: 基于PumpFun migrate指令IDL定义账户映射
        if migrate_event.mint == Pubkey::default() {
            migrate_event.mint = get_account(2);
        }
        if migrate_event.user == Pubkey::default() {
            migrate_event.user = get_account(5);
        }
        if migrate_event.bonding_curve == Pubkey::default() {
            migrate_event.bonding_curve = get_account(3);
        }
        if migrate_event.pool == Pubkey::default() {
            migrate_event.pool = get_account(9);
        }
    }
}

/// Raydium 账户填充模块
pub mod raydium {
    use super::*;

    /// 填充 Raydium CLMM Swap 事件账户
    pub fn fill_clmm_swap_accounts(
        swap_event: &mut RaydiumClmmSwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // 基于Raydium CLMM IDL swap指令账户映射
        if swap_event.pool_state == Pubkey::default() {
            swap_event.pool_state = get_account(1);
        }
        if swap_event.sender == Pubkey::default() {
            swap_event.sender = get_account(0);
        }
    }

    /// 填充 Raydium CPMM Swap 事件账户
    /// 基于Raydium CPMM swapBaseInput/swapBaseOutput指令IDL定义账户映射
    pub fn fill_cpmm_swap_accounts(
        swap_event: &mut RaydiumCpmmSwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // Raydium CPMM swap指令账户映射 (基于IDL):
        // 0: payer - 用户执行交换
        // 1: authority - 权限账户
        // 2: ammConfig - AMM配置
        // 3: poolState - 池状态
        // 4: inputTokenAccount - 输入代币账户
        // 5: outputTokenAccount - 输出代币账户
        // 6: inputVault - 输入库存
        // 7: outputVault - 输出库存
        // 8: inputTokenProgram - 输入代币程序
        // 9: outputTokenProgram - 输出代币程序
        // 10: inputTokenMint - 输入代币mint
        // 11: outputTokenMint - 输出代币mint
        // 12: observationState - 观察状态

        // 基于最新IDL swapBaseInput指令账户映射:
        // 0: payer - 用户执行交换
        // 1: authority - 权限账户
        // 2: ammConfig - AMM配置
        // 3: poolState - 池状态
        // 4: inputTokenAccount - 用户输入代币账户
        // 5: outputTokenAccount - 用户输出代币账户
        // 6: inputVault - 输入库存
        // 7: outputVault - 输出库存
        // 10: inputTokenMint - 输入代币mint
        // 11: outputTokenMint - 输出代币mint

        if swap_event.payer == Pubkey::default() {
            swap_event.payer = get_account(0);
        }
        if swap_event.authority == Pubkey::default() {
            swap_event.authority = get_account(1);
        }
        if swap_event.amm_config == Pubkey::default() {
            swap_event.amm_config = get_account(2);
        }
        if swap_event.pool_state == Pubkey::default() {
            swap_event.pool_state = get_account(3);
        }
        if swap_event.input_token_account == Pubkey::default() {
            swap_event.input_token_account = get_account(4);
        }
        if swap_event.output_token_account == Pubkey::default() {
            swap_event.output_token_account = get_account(5);
        }
        if swap_event.input_vault == Pubkey::default() {
            swap_event.input_vault = get_account(6);
        }
        if swap_event.output_vault == Pubkey::default() {
            swap_event.output_vault = get_account(7);
        }
        if swap_event.input_token_mint == Pubkey::default() {
            swap_event.input_token_mint = get_account(10);
        }
        if swap_event.output_token_mint == Pubkey::default() {
            swap_event.output_token_mint = get_account(11);
        }

    }

    /// 填充 Raydium AMM V4 Swap 事件账户
    pub fn fill_amm_v4_swap_accounts(
        swap_event: &mut RaydiumAmmV4SwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // TODO: 基于Raydium AMM V4 IDL定义账户映射
        if swap_event.amm == Pubkey::default() {
            swap_event.amm = get_account(1);
        }
        // RaydiumAmmV4SwapEvent 没有user字段，需要后续添加
    }
}

/// Orca 账户填充模块
pub mod orca {
    use super::*;

    /// 填充 Orca Whirlpool Swap 事件账户
    pub fn fill_whirlpool_swap_accounts(
        swap_event: &mut OrcaWhirlpoolSwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // 基于Orca Whirlpool swap指令IDL账户映射:
        // 0: tokenProgram - SPL代币程序 (常量)
        // 1: tokenAuthority - 代币权限
        // 2: whirlpool - 池状态
        // 3: tokenOwnerAccountA - 用户代币A账户
        // 4: tokenVaultA - 池代币A库存
        // 5: tokenOwnerAccountB - 用户代币B账户
        // 6: tokenVaultB - 池代币B库存
        // 7: tickArray0 - tick数组0
        // 8: tickArray1 - tick数组1
        // 9: tickArray2 - tick数组2

        if swap_event.token_authority == Pubkey::default() {
            swap_event.token_authority = get_account(1);
        }
        if swap_event.whirlpool == Pubkey::default() {
            swap_event.whirlpool = get_account(2);
        }
        if swap_event.token_owner_account_a == Pubkey::default() {
            swap_event.token_owner_account_a = get_account(3);
        }
        if swap_event.token_vault_a == Pubkey::default() {
            swap_event.token_vault_a = get_account(4);
        }
        if swap_event.token_owner_account_b == Pubkey::default() {
            swap_event.token_owner_account_b = get_account(5);
        }
        if swap_event.token_vault_b == Pubkey::default() {
            swap_event.token_vault_b = get_account(6);
        }
        if swap_event.tick_array_0 == Pubkey::default() {
            swap_event.tick_array_0 = get_account(7);
        }
        if swap_event.tick_array_1 == Pubkey::default() {
            swap_event.tick_array_1 = get_account(8);
        }
        if swap_event.tick_array_2 == Pubkey::default() {
            swap_event.tick_array_2 = get_account(9);
        }
    }
}

/// Meteora 账户填充模块
pub mod meteora {
    use super::*;

    /// 填充 Meteora Pools Swap 事件账户
    pub fn fill_pools_swap_accounts(
        swap_event: &mut MeteoraPoolsSwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // TODO: 基于Meteora Pools IDL定义账户映射
        // 当前MeteoraPoolsSwapEvent基于IDL事件字段，账户信息从日志中填充
        // 这里可以填充指令账户字段（如果有的话）
        // MeteoraPoolsSwapEvent没有user字段，只有IDL事件字段
    }

    /// 填充 Meteora DAMM V2 Swap 事件账户
    pub fn fill_damm_v2_swap_accounts(
        swap_event: &mut MeteoraDammV2SwapEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // 基于Meteora DAMM V2 IDL swap指令账户映射
        if swap_event.lb_pair == Pubkey::default() {
            swap_event.lb_pair = get_account(1);
        }
        if swap_event.from == Pubkey::default() {
            swap_event.from = get_account(0);
        }
    }
}

/// Bonk 账户填充模块
pub mod bonk {
    use super::*;

    /// 填充 Bonk Trade 事件账户
    pub fn fill_trade_accounts(
        trade_event: &mut BonkTradeEvent,
        get_account: &AccountGetter<'_>,
    ) {
        // 基于Bonk IDL swap指令账户映射
        if trade_event.user == Pubkey::default() {
            trade_event.user = get_account(0);
        }
        if trade_event.pool_state == Pubkey::default() {
            trade_event.pool_state = get_account(1);
        }
    }
}