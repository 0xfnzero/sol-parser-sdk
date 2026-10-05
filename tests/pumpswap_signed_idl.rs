//! Generate wire data from the official IDL field order, independently of decoder offsets.
use base64::Engine;
use serde_json::Value;
use sol_parser_sdk::{
    accounts::{pumpswap, token::AccountData},
    core::{events::PumpSwapBuyEvent, merger::merge_events},
    instr::pump_amm_inner,
    logs::pump_amm,
    DexEvent, EventMetadata,
};
use solana_sdk::pubkey::Pubkey;

fn idl_payload(name: &str, virtual_reserve: i128) -> Vec<u8> {
    let idl: Value = serde_json::from_str(include_str!("../idl/pump_amm.json")).unwrap();
    let fields = idl["types"].as_array().unwrap().iter().find(|ty| ty["name"] == name).unwrap()
        ["type"]["fields"]
        .as_array()
        .unwrap();
    let mut bytes = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let value = match field["name"].as_str().unwrap() {
            "pool_quote_token_reserves" => 1_000,
            "pool_base_token_reserves" => 2_000,
            _ => index as u64 + 1,
        };
        if let Some(array) = field["type"]["array"].as_array() {
            assert_eq!(array[0], "pubkey");
            for index in 0..array[1].as_u64().unwrap() {
                bytes.extend_from_slice(&[value as u8 + index as u8; 32]);
            }
            continue;
        }
        match field["type"].as_str().unwrap() {
            "u8" => bytes.push(value as u8),
            "u16" => bytes.extend_from_slice(&(value as u16).to_le_bytes()),
            "u64" => bytes.extend_from_slice(&value.to_le_bytes()),
            "i64" => bytes.extend_from_slice(&(-(value as i64)).to_le_bytes()),
            "i128" => bytes.extend_from_slice(&virtual_reserve.to_le_bytes()),
            "bool" => bytes.push(1),
            "pubkey" => bytes.extend_from_slice(&[value as u8; 32]),
            "string" => {
                let name = b"buy_exact_quote_in";
                bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
                bytes.extend_from_slice(name);
            }
            other => panic!("unsupported IDL fixture type: {other}"),
        }
    }
    bytes
}

#[test]
fn pumpswap_pool_signed_reserves_follow_official_idl() {
    for value in [i128::MIN, -500, 0, 500, i128::MAX] {
        let mut data = pumpswap::discriminators::POOL_ACCOUNT.to_vec();
        data.extend(idl_payload("Pool", value));
        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            data,
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        };
        let DexEvent::PumpSwapPoolAccount(event) =
            pumpswap::parse_pool(&account, EventMetadata::default()).unwrap()
        else {
            panic!("expected Pool account")
        };
        assert_eq!(event.pool.virtual_quote_reserves, value);
        assert_eq!(event.pool.creator_fee_bps, 14);
        assert!(event.pool.can_edit_creator_fee);
        assert!(event.pool.is_holder_reward);
    }
}

#[test]
fn pumpswap_trade_signed_reserves_survive_log_cpi_merge_and_json() {
    for value in [i128::MIN, -500, 0, 500, i128::MAX] {
        for name in ["BuyEvent", "SellEvent"] {
            let bytes = idl_payload(name, value);
            let (raw_event, discriminator) = if name == "BuyEvent" {
                (
                    pump_amm::parse_buy_from_data(&bytes, EventMetadata::default()).unwrap(),
                    pump_amm_inner::discriminators::BUY,
                )
            } else {
                (
                    pump_amm::parse_sell_from_data(&bytes, EventMetadata::default()).unwrap(),
                    pump_amm_inner::discriminators::SELL,
                )
            };
            let mut log_bytes = discriminator[8..].to_vec();
            log_bytes.extend_from_slice(&bytes);
            let log = format!(
                "Program data: {}",
                base64::engine::general_purpose::STANDARD.encode(log_bytes)
            );
            let mut event = pump_amm::parse_log(&log, Default::default(), 123, 0, None, 0).unwrap();
            assert_eq!(std::mem::discriminant(&event), std::mem::discriminant(&raw_event));
            let inner = pump_amm_inner::parse_pumpswap_inner_instruction(
                &discriminator,
                &bytes,
                EventMetadata::default(),
            )
            .unwrap();
            merge_events(&mut event, inner);
            let (reserve, raw, holder_bps) = match &event {
                DexEvent::PumpSwapBuy(e) => {
                    let json = serde_json::to_string(e).unwrap();
                    let roundtrip: PumpSwapBuyEvent = serde_json::from_str(&json).unwrap();
                    assert_eq!(roundtrip.virtual_quote_reserves, value);
                    (e.virtual_quote_reserves, e.pool_quote_token_reserves, e.holder_rewards_bps)
                }
                DexEvent::PumpSwapSell(e) => {
                    let json = serde_json::to_string(e).unwrap();
                    let roundtrip: sol_parser_sdk::core::events::PumpSwapSellEvent =
                        serde_json::from_str(&json).unwrap();
                    assert_eq!(roundtrip.virtual_quote_reserves, value);
                    (e.virtual_quote_reserves, e.pool_quote_token_reserves, e.holder_rewards_bps)
                }
                _ => panic!("expected trade"),
            };
            assert_eq!(reserve, value);
            assert_eq!(raw, 1_000);
            assert!(holder_bps > 0);
        }
    }
}

#[test]
fn pumpswap_global_config_appended_controls_follow_official_idl() {
    let mut data = pumpswap::discriminators::GLOBAL_CONFIG_ACCOUNT.to_vec();
    data.extend(idl_payload("GlobalConfig", 0));
    let account = AccountData {
        pubkey: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        data,
        executable: false,
        lamports: 1,
        rent_epoch: 0,
    };
    let DexEvent::PumpSwapGlobalConfigAccount(event) =
        pumpswap::parse_global_config(&account, EventMetadata::default()).unwrap()
    else {
        panic!("expected global config")
    };
    assert!(event.global_config.is_cashback_enabled);
    assert_eq!(event.global_config.buyback_basis_points, 14);
    assert!(event.global_config.boost_enabled);
    assert!(event.global_config.creator_fee_configurable);
    assert_eq!(event.global_config.max_configurable_creator_fee_bps, 18);
}
