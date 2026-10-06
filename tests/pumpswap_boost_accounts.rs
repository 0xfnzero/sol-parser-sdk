use sol_parser_sdk::{
    core::{account_dispatcher::fill_accounts_with_owned_keys, events::PumpSwapBuyEvent},
    DexEvent,
};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, Message, Transaction, TransactionStatusMeta,
};

fn fixture(
    discs: &[[u8; 8]],
) -> (Vec<Pubkey>, Option<Transaction>, TransactionStatusMeta, HashMap<Pubkey, Vec<(i32, i32)>>) {
    let keys: Vec<_> = (0..28).map(|_| Pubkey::new_unique()).collect();
    let program = sol_parser_sdk::grpc::program_ids::PUMPSWAP_PROGRAM;
    let mut account_keys: Vec<_> = keys.iter().map(|key| key.to_bytes().to_vec()).collect();
    account_keys.push(program.to_bytes().to_vec());
    let instructions = discs
        .iter()
        .enumerate()
        .map(|(index, disc)| {
            let mut accounts: Vec<u8> = (0..23).collect();
            if index == 1 {
                accounts[1] = 23;
                accounts[5] = 24;
                accounts[6] = 25;
            }
            CompiledInstruction { program_id_index: 28, accounts, data: disc.to_vec() }
        })
        .collect();
    let tx = Some(Transaction {
        message: Some(Message { account_keys, instructions, ..Default::default() }),
        ..Default::default()
    });
    let invokes = HashMap::from([(program, (0..discs.len()).map(|i| (i as i32, -1)).collect())]);
    (keys, tx, TransactionStatusMeta::default(), invokes)
}

#[test]
fn repeated_buy_must_use_the_events_user() {
    use sol_parser_sdk::instr::pump_amm::discriminators::BUY;
    let (keys, tx, meta, invokes) = fixture(&[BUY, BUY]);
    let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        pool: keys[0],
        user: keys[23],
        ..Default::default()
    });
    fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
    let DexEvent::PumpSwapBuy(event) = event else { unreachable!() };
    assert_eq!(
        event.user_base_token_account, keys[24],
        "second user must not inherit first user's token account"
    );
}

#[test]
fn buy_must_not_fall_back_to_sell_layout() {
    use sol_parser_sdk::instr::pump_amm::discriminators::SELL;
    let (keys, tx, meta, invokes) = fixture(&[SELL]);
    let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        pool: keys[0],
        user: keys[1],
        ..Default::default()
    });
    fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
    let DexEvent::PumpSwapBuy(event) = event else { unreachable!() };
    assert_eq!(
        event.user_base_token_account,
        Pubkey::default(),
        "a missing buy invoke must stay unresolved"
    );
}

#[test]
fn regular_buy_after_boost_must_keep_regular_buy_accounts() {
    use sol_parser_sdk::instr::pump_amm::discriminators::BUY;
    const BOOST_BUY_AND_BURN: [u8; 8] = [105, 68, 6, 175, 0, 7, 35, 162];
    let (keys, mut tx, meta, invokes) = fixture(&[BOOST_BUY_AND_BURN, BUY]);
    tx.as_mut().unwrap().message.as_mut().unwrap().instructions[0].accounts.truncate(13);
    let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        pool: keys[0],
        user: keys[23],
        ..Default::default()
    });
    fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
    let DexEvent::PumpSwapBuy(event) = event else { unreachable!() };
    assert_eq!(
        event.pool_base_token_account, keys[7],
        "regular buy must not receive boost's slot-5 vault"
    );
    assert_eq!(event.user_base_token_account, keys[24]);
}

#[test]
fn boost_matches_vault_authority_for_outer_and_inner_invokes() {
    use sol_parser_sdk::instr::pump_amm::discriminators::BOOST_BUY_AND_BURN;
    use yellowstone_grpc_proto::prelude::{InnerInstruction, InnerInstructions};
    for inner in [false, true] {
        let (keys, mut tx, mut meta, mut invokes) = fixture(&[BOOST_BUY_AND_BURN]);
        let message = tx.as_mut().unwrap().message.as_mut().unwrap();
        message.instructions[0].accounts.truncate(13);
        if inner {
            let ix = message.instructions.remove(0);
            meta.inner_instructions = vec![InnerInstructions {
                index: 0,
                instructions: vec![InnerInstruction {
                    program_id_index: ix.program_id_index,
                    accounts: ix.accounts,
                    data: ix.data,
                    ..Default::default()
                }],
            }];
            invokes.values_mut().next().unwrap()[0] = (0, 0);
        }
        for (user, matched) in [(keys[7], true), (keys[1], false), (Pubkey::default(), true)] {
            let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                pool: keys[0],
                user,
                ..Default::default()
            });
            fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
            let DexEvent::PumpSwapBuy(event) = event else { unreachable!() };
            if matched {
                assert_eq!((event.base_mint, event.quote_mint), (keys[3], keys[4]));
                assert_eq!(
                    (event.pool_base_token_account, event.pool_quote_token_account),
                    (keys[5], keys[6])
                );
                assert_eq!(
                    (event.base_token_program, event.quote_token_program),
                    (keys[9], keys[10])
                );
            } else {
                assert_eq!(event.base_mint, Pubkey::default());
            }
            assert_eq!(event.user_base_token_account, Pubkey::default());
            assert_eq!(event.protocol_fee_recipient, Pubkey::default());
            assert_eq!(event.coin_creator_vault_ata, Pubkey::default());
            assert_eq!(event.user, user);
        }
    }
}

#[test]
fn ambiguous_and_short_boost_invokes_stay_unresolved() {
    use sol_parser_sdk::instr::pump_amm::discriminators::BOOST_BUY_AND_BURN;
    for count in [12, 13] {
        let (keys, mut tx, meta, invokes) = fixture(&[BOOST_BUY_AND_BURN, BOOST_BUY_AND_BURN]);
        for ix in &mut tx.as_mut().unwrap().message.as_mut().unwrap().instructions {
            ix.accounts.truncate(count);
        }
        let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
            pool: keys[0],
            user: keys[7],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut event, &meta, &tx, &invokes);
        let DexEvent::PumpSwapBuy(event) = event else { unreachable!() };
        assert_eq!(event.base_mint, Pubkey::default());
    }
}

fn replay_paths(fixture: &str) -> [Vec<DexEvent>; 3] {
    use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
    use yellowstone_grpc_proto::prelude::{
        SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
    };
    let tx: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_str(fixture).unwrap();
    let rpc = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
    let (meta, transaction) = sol_parser_sdk::convert_rpc_to_grpc(&tx).unwrap();
    let update = SubscribeUpdateTransaction {
        slot: tx.slot,
        transaction: Some(SubscribeUpdateTransactionInfo {
            signature: transaction.signatures[0].clone(),
            transaction: Some(transaction),
            meta: Some(meta),
            ..Default::default()
        }),
    };
    let time = tx.block_time.map(|seconds| seconds * 1_000_000);
    [
        rpc,
        sol_parser_sdk::grpc::parse_subscribe_update_transaction(&update, 0, time, None),
        sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update, 0, time, None,
        ),
    ]
}

#[test]
fn mainnet_boost_uses_real_vaults_and_token_programs_on_all_paths() {
    for events in replay_paths(include_str!("fixtures/pumpswap_boost_buy_and_burn.json")) {
        let buys: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                DexEvent::PumpSwapBuy(buy) => Some(buy),
                _ => None,
            })
            .collect();
        assert_eq!(buys.len(), 1);
        let buy = buys[0];
        for (actual, expected) in [
            (buy.pool, "E5WvAEhX1LKCtCz9BuzBZ4zwEzt4kYS1Du4jxGK1R3Lc"),
            (buy.user, "9SNoU8GVBT7VTg7wfyTbtrgPb53fQTDYmg88EMaGjUQh"),
            (buy.base_mint, "7rmPZgtU22AN1W3kHy8ASw76v46fp8kLgzadnPcEpump"),
            (buy.pool_base_token_account, "KbW4EmdCjfbJefaTDJJ6ww2Gk4Z7nseeVN9KaKZNueE"),
            (buy.pool_quote_token_account, "3Qia2MrJcBaF5ttA4XjqEcoZUr53g58XxTmJopHbMQJG"),
            (buy.base_token_program, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
            (buy.quote_token_program, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        ] {
            assert_eq!(actual.to_string(), expected);
        }
    }
}

#[test]
fn mainnet_precompile_does_not_shift_pumpswap_sell_accounts() {
    for events in replay_paths(include_str!("fixtures/pumpswap_precompile_sell.json")) {
        let sells: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                DexEvent::PumpSwapSell(sell)
                    if sell.pool.to_string() == "2NtHe5gwQ1grT3Y89Kf3dRZyWDnTVYk4f4xV9WxuZJBZ" =>
                {
                    Some(sell)
                }
                _ => None,
            })
            .collect();
        assert_eq!(sells.len(), 1);
        assert_eq!(sells[0].base_mint.to_string(), "7hLzF2ahWJKwRNrW4jGSteWBGWJ2eVMBi6BJFvaquant");
    }
}
