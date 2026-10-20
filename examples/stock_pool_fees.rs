//! Offline fee audit: cargo run --example stock_pool_fees -- SNAPSHOT.json
//! Static rates and token transfer tax are decoded separately. Dynamic swap
//! rates are unknown, not zero; protocol fee shares are not added a second time.
use anyhow::Context;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sol_parser_sdk::{
    accounts::{orca_whirlpool, raydium_clmm, token::AccountData},
    DexEvent,
};
use spl_token_2022::{
    extension::{
        transfer_fee::TransferFeeConfig, BaseStateWithExtensions, ExtensionType,
        StateWithExtensions,
    },
    state::Mint,
};

fn account(snapshot: &Value, key: &str) -> anyhow::Result<AccountData> {
    let a = &snapshot["accounts"][key];
    Ok(AccountData {
        pubkey: key.parse()?,
        owner: a["owner"].as_str().ok_or_else(|| anyhow::anyhow!("missing owner"))?.parse()?,
        data: STANDARD
            .decode(a["data"][0].as_str().ok_or_else(|| anyhow::anyhow!("missing data"))?)?,
        executable: false,
        lamports: 0,
        rent_epoch: 0,
    })
}
fn transfer_tax(a: &AccountData, epoch: u64) -> anyhow::Result<Value> {
    if a.owner.to_string() == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
        anyhow::ensure!(a.data.len() == 82 && a.data[45] == 1, "invalid SPL mint");
        return Ok(
            json!({"mint":a.pubkey.to_string(),"basis_points":0,"maximum_fee_raw":0,"transfer_hook":false}),
        );
    }
    anyhow::ensure!(
        a.owner.to_string() == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        "unknown token program"
    );
    let mint = StateWithExtensions::<Mint>::unpack(&a.data)?;
    let extensions = mint.get_extension_types()?;
    let fee = if extensions.contains(&ExtensionType::TransferFeeConfig) {
        let config = mint.get_extension::<TransferFeeConfig>()?;
        let rate = config.get_epoch_fee(epoch);
        (u16::from(rate.transfer_fee_basis_points), u64::from(rate.maximum_fee))
    } else {
        (0, 0)
    };
    let hook = spl_token_2022::extension::transfer_hook::get_program_id(&mint);
    Ok(json!({"mint":a.pubkey.to_string(),"basis_points":fee.0,"maximum_fee_raw":fee.1,
        "transfer_hook":hook.is_some(),"transfer_hook_program":hook.map(|p|p.to_string())}))
}
fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("snapshot path required"))?;
    let snapshot: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let snapshot_slot = snapshot["slot"].as_u64().context("snapshot slot missing")?;
    let clock = account(&snapshot, "SysvarC1ock11111111111111111111111111111111")?;
    anyhow::ensure!(
        clock.owner == solana_sdk::pubkey!("Sysvar1111111111111111111111111111111111111"),
        "invalid Clock owner"
    );
    anyhow::ensure!(clock.data.len() >= 40, "truncated clock");
    anyhow::ensure!(
        u64::from_le_bytes(clock.data[..8].try_into()?) == snapshot_slot,
        "Clock does not match the snapshot slot"
    );
    let epoch = u64::from_le_bytes(clock.data[16..24].try_into()?);
    let mut reports = Vec::new();
    for hint in snapshot["hints"].as_array().ok_or_else(|| anyhow::anyhow!("missing hints"))? {
        let field = |name: &str| hint[name].as_str().context("pool hint key missing");
        let pool = account(&snapshot, field("pool")?)?;
        let owner = pool.owner.to_string();
        let (base_bps, dynamic) = match owner.as_str() {
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
                let Some(DexEvent::RaydiumClmmPoolStateAccount(p)) =
                    raydium_clmm::parse_pool_state(&pool, Default::default())
                else {
                    anyhow::bail!("invalid CLMM pool")
                };
                let config = account(&snapshot, &p.pool_state.amm_config.to_string())?;
                anyhow::ensure!(config.owner == pool.owner, "config owner mismatch");
                let Some(DexEvent::RaydiumClmmAmmConfigAccount(c)) =
                    raydium_clmm::parse_amm_config(&config, Default::default())
                else {
                    anyhow::bail!("invalid CLMM config")
                };
                (
                    f64::from(c.amm_config.trade_fee_rate) / 100.0,
                    p.pool_state.dynamic_fee_info.dynamic_fee_control != 0,
                )
            }
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
                let Some(DexEvent::OrcaWhirlpoolAccount(p)) =
                    orca_whirlpool::parse_whirlpool(&pool, Default::default())
                else {
                    anyhow::bail!("invalid Whirlpool")
                };
                (
                    f64::from(p.whirlpool.fee_rate) / 100.0,
                    u16::from_le_bytes(pool.data[43..45].try_into()?) != p.whirlpool.tick_spacing,
                )
            }
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
                let d = &pool.data;
                anyhow::ensure!(
                    d.len() >= 584 && d[..8] == [33, 11, 49, 98, 181, 101, 177, 13],
                    "invalid DLMM pool"
                );
                let factor = u16::from_le_bytes(d[8..10].try_into()?);
                let step = u16::from_le_bytes(d[80..82].try_into()?);
                let power = d[34];
                anyhow::ensure!(power <= 9, "unsupported DLMM base fee power");
                // Official StaticParameters formula; precision is 1e9.
                (
                    f64::from(factor) * f64::from(step) * 10_f64.powi(i32::from(power)) / 10_000.0,
                    u32::from_le_bytes(d[16..20].try_into()?) != 0,
                )
            }
            _ => continue,
        };
        anyhow::ensure!(base_bps.is_finite() && (0.0..10000.0).contains(&base_bps), "invalid rate");
        let stock = transfer_tax(&account(&snapshot, field("output_mint")?)?, epoch)?;
        let quote = transfer_tax(&account(&snapshot, field("input_mint")?)?, epoch)?;
        reports.push(
            json!({"pool":pool.pubkey.to_string(),"snapshot_slot":snapshot["slot"],"epoch":epoch,
            "unix_timestamp":i64::from_le_bytes(clock.data[32..40].try_into()?),
            "base_swap_fee_bps":base_bps,"swap_fee_bps":if dynamic {None} else {Some(base_bps)},
            "dynamic_swap_fee":dynamic,"stock_transfer_tax":stock,"quote_transfer_tax":quote}),
        );
    }
    println!("{}", serde_json::to_string(&reports)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spl_token_2022::extension::{
        transfer_fee::TransferFee, transfer_hook::TransferHook, BaseStateWithExtensionsMut,
        StateWithExtensionsMut,
    };

    #[test]
    fn epoch_selects_transfer_tax_and_preserves_raw_cap_and_disabled_hook() {
        let len = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::TransferFeeConfig,
            ExtensionType::TransferHook,
        ])
        .unwrap();
        let mut data = vec![0; len];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
        let fee = state.init_extension::<TransferFeeConfig>(true).unwrap();
        fee.older_transfer_fee = TransferFee {
            epoch: 1u64.into(),
            maximum_fee: 1000u64.into(),
            transfer_fee_basis_points: 25u16.into(),
        };
        fee.newer_transfer_fee = TransferFee {
            epoch: 10u64.into(),
            maximum_fee: 2000u64.into(),
            transfer_fee_basis_points: 50u16.into(),
        };
        state.init_extension::<TransferHook>(true).unwrap();
        state.base.is_initialized = true;
        state.base.decimals = 8;
        state.pack_base();
        state.init_account_type().unwrap();
        let account = AccountData {
            pubkey: Default::default(),
            owner: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".parse().unwrap(),
            data,
            executable: false,
            lamports: 0,
            rent_epoch: 0,
        };
        let old = transfer_tax(&account, 9).unwrap();
        let new = transfer_tax(&account, 10).unwrap();
        assert_eq!(old["basis_points"], 25);
        assert_eq!(old["maximum_fee_raw"], 1000);
        assert_eq!(new["basis_points"], 50);
        assert_eq!(new["maximum_fee_raw"], 2000);
        assert_eq!(new["transfer_hook"], false);
    }
}
