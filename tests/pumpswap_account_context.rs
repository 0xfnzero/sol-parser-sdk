use serde_json::Value;
use sol_parser_sdk::{
    core::{
        account_dispatcher::fill_accounts_with_owned_keys,
        events::{PumpSwapBuyEvent, PumpSwapSellEvent},
    },
    DexEvent,
};
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, str::FromStr};
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, Transaction,
    TransactionStatusMeta,
};

#[test]
fn mainnet_trade_context_fills_log_only_events() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_upgrade/account_context.json")).unwrap();
    let program = Pubkey::from_str(fixture["program"].as_str().unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let keys: Vec<Pubkey> = case["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| Pubkey::from_str(k.as_str().unwrap()).unwrap())
            .collect();
        let instructions: Vec<CompiledInstruction> = case["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                let hex = i["data"].as_str().unwrap();
                CompiledInstruction {
                    program_id_index: keys.iter().position(|k| *k == program).unwrap() as u32,
                    accounts: i["accounts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_u64().unwrap() as u8)
                        .collect(),
                    data: (0..hex.len())
                        .step_by(2)
                        .map(|n| u8::from_str_radix(&hex[n..n + 2], 16).unwrap())
                        .collect(),
                }
            })
            .collect();
        for inner in [false, true] {
            let mut meta = TransactionStatusMeta::default();
            let tx = Some(Transaction {
                message: Some(Message {
                    account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                    instructions: if inner { vec![] } else { instructions.clone() },
                    ..Default::default()
                }),
                ..Default::default()
            });
            let invokes = if inner {
                meta.inner_instructions.push(InnerInstructions {
                    index: 0,
                    instructions: instructions
                        .iter()
                        .map(|i| InnerInstruction {
                            program_id_index: i.program_id_index,
                            accounts: i.accounts.clone(),
                            data: i.data.clone(),
                            ..Default::default()
                        })
                        .collect(),
                });
                (0..instructions.len()).map(|i| (0, i as i32)).collect()
            } else {
                (0..instructions.len()).map(|i| (i as i32, -1)).collect()
            };
            let invokes = HashMap::from([(program, invokes)]);
            for expected in case["events"].as_array().unwrap() {
                let pool = Pubkey::from_str(expected["pool"].as_str().unwrap()).unwrap();
                let user = Pubkey::from_str(expected["user"].as_str().unwrap()).unwrap();
                let mut event = if expected["buy"].as_bool().unwrap() {
                    DexEvent::PumpSwapBuy(PumpSwapBuyEvent { pool, user, ..Default::default() })
                } else {
                    DexEvent::PumpSwapSell(PumpSwapSellEvent { pool, user, ..Default::default() })
                };
                fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
                let json = serde_json::to_value(&event).unwrap();
                let body = json.as_object().unwrap().values().next().unwrap();
                for (field, key) in expected["expected"].as_object().unwrap() {
                    let actual = if let Some(s) = body[field].as_str() {
                        s.to_owned()
                    } else {
                        let bytes: [u8; 32] = serde_json::from_value(body[field].clone()).unwrap();
                        Pubkey::new_from_array(bytes).to_string()
                    };
                    assert_eq!(
                        actual,
                        key.as_str().unwrap(),
                        "{} {field} inner={inner}",
                        case["signature"]
                    );
                }
            }
        }
    }
}

#[test]
fn unmatched_or_ambiguous_context_stays_unresolved() {
    let program = sol_parser_sdk::grpc::program_ids::PUMPSWAP_PROGRAM;
    let keys: Vec<_> = (0..17).map(|_| Pubkey::new_unique()).collect();
    for mode in ["duplicate", "wrong_user", "wrong_direction", "truncated", "foreign_instruction"] {
        let mut ix = CompiledInstruction {
            accounts: (0..17).collect(),
            data: vec![184, 23, 238, 97, 103, 197, 211, 61],
            ..Default::default()
        };
        if mode == "truncated" {
            ix.accounts.truncate(16);
        }
        if mode == "foreign_instruction" {
            ix.data = vec![0; 24];
        }
        let mut instructions = vec![ix.clone()];
        if mode == "duplicate" {
            instructions.push(ix);
        }
        let invokes =
            HashMap::from([(program, (0..instructions.len()).map(|i| (i as i32, -1)).collect())]);
        let tx = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions,
                ..Default::default()
            }),
            ..Default::default()
        });
        let user = if mode == "wrong_user" { keys[2] } else { keys[1] };
        let mut event = if mode == "wrong_direction" {
            DexEvent::PumpSwapSell(PumpSwapSellEvent { pool: keys[0], user, ..Default::default() })
        } else {
            DexEvent::PumpSwapBuy(PumpSwapBuyEvent { pool: keys[0], user, ..Default::default() })
        };
        fill_accounts_with_owned_keys(&mut event, &TransactionStatusMeta::default(), &tx, &invokes);
        let account = match event {
            DexEvent::PumpSwapBuy(e) => e.user_base_token_account,
            DexEvent::PumpSwapSell(e) => e.user_base_token_account,
            _ => unreachable!(),
        };
        assert_eq!(account, Pubkey::default(), "{mode}");
    }
}
