use sol_parser_sdk::{
    accounts::{parse_account_unified, raydium_cpmm::discriminators, AccountData},
    core::events::{DexEvent, EventMetadata},
    grpc::{EventType, EventTypeFilter},
    instr::{parse_instruction_unified, program_ids::RAYDIUM_CPMM_PROGRAM_ID, raydium_cpmm},
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[test]
fn custom_share_account_routes_through_filtered_account_parser() {
    let mut data = vec![0; 145];
    data[..8].copy_from_slice(discriminators::CREATOR_FEE_SHARE);
    data[8] = 254;
    let creator = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    data[9..41].copy_from_slice(creator.as_ref());
    data[41..73].copy_from_slice(config.as_ref());
    data[73..81].copy_from_slice(&300_000u64.to_le_bytes());
    let mut account = AccountData {
        pubkey: Pubkey::new_unique(),
        owner: RAYDIUM_CPMM_PROGRAM_ID,
        data,
        executable: false,
        lamports: 1,
        rent_epoch: 0,
    };
    let filter = EventTypeFilter::include_only(vec![EventType::AccountRaydiumCpmmCreatorFeeShare]);
    let DexEvent::RaydiumCpmmCreatorFeeShareAccount(event) =
        parse_account_unified(&account, EventMetadata::default(), Some(&filter)).unwrap()
    else {
        panic!("share account");
    };
    assert_eq!(event.creator_fee_share.bump, 254);
    assert_eq!(event.creator_fee_share.creator, creator);
    assert_eq!(event.creator_fee_share.amm_config, config);
    assert_eq!(event.creator_fee_share.share_rate, 300_000);
    let mismatch = EventTypeFilter::include_only(vec![EventType::AccountRaydiumCpmmAmmConfig]);
    assert!(parse_account_unified(&account, EventMetadata::default(), Some(&mismatch)).is_none());
    let complete = account.data.clone();
    for len in [0, 7, 8, 73, 144] {
        account.data = complete[..len].to_vec();
        assert!(parse_account_unified(&account, EventMetadata::default(), Some(&filter)).is_none());
    }
}

#[test]
fn collection_routes_through_filtered_instruction_parser() {
    let keys: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmCollectCreatorFee]);
    for (disc, len) in [
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE, 15),
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS, 16),
    ] {
        let event = parse_instruction_unified(
            &disc,
            &keys[..len],
            Signature::default(),
            1,
            0,
            None,
            0,
            Some(&filter),
            &RAYDIUM_CPMM_PROGRAM_ID,
        )
        .unwrap();
        assert!(matches!(event, DexEvent::RaydiumCpmmCollectCreatorFee(_)));
        let mismatch = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]);
        assert!(parse_instruction_unified(
            &disc,
            &keys[..len],
            Signature::default(),
            1,
            0,
            None,
            0,
            Some(&mismatch),
            &RAYDIUM_CPMM_PROGRAM_ID
        )
        .is_none());
    }
}

#[test]
fn collection_field_positions_and_truncation_are_independent_of_trade_builder() {
    let keys: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
    for (disc, permissionless, required) in [
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE, false, 15),
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS, true, 16),
    ] {
        for length in 0..required {
            assert!(raydium_cpmm::parse_instruction(
                &disc,
                &keys[..length],
                Signature::default(),
                1,
                0,
                None
            )
            .is_none());
        }
        for length in 0..8 {
            assert!(raydium_cpmm::parse_instruction(
                &disc[..length],
                &keys,
                Signature::default(),
                1,
                0,
                None
            )
            .is_none());
        }
        // Extra remaining accounts must not shift the documented account positions.
        let DexEvent::RaydiumCpmmCollectCreatorFee(e) =
            raydium_cpmm::parse_instruction(&disc, &keys, Signature::default(), 1, 0, None)
                .unwrap()
        else {
            panic!("collection")
        };
        assert_eq!(e.permissionless, permissionless);
        assert_eq!(e.payer, keys[0]);
        assert_eq!(e.creator, keys[usize::from(permissionless)]);
        assert_eq!(e.authority, keys[if permissionless { 2 } else { 1 }]);
        assert_eq!(e.pool_state, keys[if permissionless { 3 } else { 2 }]);
        assert_eq!(e.amm_config, keys[if permissionless { 14 } else { 3 }]);
        assert_eq!(e.token_0_vault, keys[4]);
        assert_eq!(e.token_1_vault, keys[5]);
        assert_eq!(e.vault_0_mint, keys[6]);
        assert_eq!(e.vault_1_mint, keys[7]);
        assert_eq!(e.creator_token_0, keys[8]);
        assert_eq!(e.creator_token_1, keys[9]);
        assert_eq!(e.token_0_program, keys[10]);
        assert_eq!(e.token_1_program, keys[11]);
        assert_eq!(e.associated_token_program, keys[12]);
        assert_eq!(e.system_program, keys[13]);
        assert_eq!(e.creator_fee_share, keys[if permissionless { 15 } else { 14 }]);
    }
}

#[test]
fn shred_transaction_path_parses_collection_without_rpc_or_executed_balances() {
    use sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        message::Message,
        transaction::{Transaction, VersionedTransaction},
    };
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmCollectCreatorFee]);
    for (disc, permissionless, length) in [
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE, false, 15),
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS, true, 16),
    ] {
        let keys: Vec<_> = (0..length).map(|_| Pubkey::new_unique()).collect();
        let ix = Instruction {
            program_id: RAYDIUM_CPMM_PROGRAM_ID,
            data: disc.to_vec(),
            accounts: keys
                .iter()
                .enumerate()
                .map(|(i, key)| AccountMeta::new_readonly(*key, i == 0))
                .collect(),
        };
        let transaction = VersionedTransaction::from(Transaction::new_unsigned(Message::new(
            &[ix],
            Some(&keys[0]),
        )));
        let mut events = Vec::new();
        parse_transaction_dex_events_with_filter(
            &transaction,
            Signature::default(),
            123,
            0,
            0,
            Some(&filter),
            &mut events,
        );
        assert_eq!(events.len(), 1);
        let DexEvent::RaydiumCpmmCollectCreatorFee(e) = &events[0] else { panic!("collection") };
        assert_eq!(e.permissionless, permissionless);
        assert_eq!(e.pool_state, keys[if permissionless { 3 } else { 2 }]);
        assert_eq!(e.amm_config, keys[if permissionless { 14 } else { 3 }]);
        assert_eq!(e.creator_fee_share, *keys.last().unwrap());
        let mismatch = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]);
        events.clear();
        parse_transaction_dex_events_with_filter(
            &transaction,
            Signature::default(),
            123,
            0,
            0,
            Some(&mismatch),
            &mut events,
        );
        assert!(events.is_empty());
    }
}

#[test]
fn amm_config_rate_reuses_padding_without_changing_account_size() {
    use sol_parser_sdk::accounts::raydium_cpmm::{parse_amm_config, AMM_CONFIG_SIZE};
    let mut data = vec![0; 236];
    data[..8].copy_from_slice(discriminators::AMM_CONFIG);
    data[108..116].copy_from_slice(&123u64.to_le_bytes());
    data[116..124].copy_from_slice(&200_000u64.to_le_bytes());
    data[228..236].copy_from_slice(&999u64.to_le_bytes());
    assert_eq!(8 + AMM_CONFIG_SIZE, 236);
    let account = AccountData {
        pubkey: Pubkey::new_unique(),
        owner: RAYDIUM_CPMM_PROGRAM_ID,
        data: data.clone(),
        executable: false,
        lamports: 1,
        rent_epoch: 0,
    };
    let DexEvent::RaydiumCpmmAmmConfigAccount(e) =
        parse_amm_config(&account, EventMetadata::default()).unwrap()
    else {
        panic!("config")
    };
    assert_eq!(e.amm_config.creator_fee_rate, 123);
    assert_eq!(e.amm_config.creator_fee_share_rate, 200_000);
    assert_eq!(e.amm_config.padding[13], 999);
    for count in 0..236 {
        let mut incomplete = account.clone();
        incomplete.data = data[..count].to_vec();
        assert!(parse_amm_config(&incomplete, EventMetadata::default()).is_none());
    }
}

#[test]
fn failed_grpc_collection_is_never_emitted_as_a_successful_event() {
    use sol_parser_sdk::grpc::{
        parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency,
    };
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, Message, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
        Transaction, TransactionError, TransactionStatusMeta,
    };
    let parsers =
        [parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency];
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmCollectCreatorFee]);
    for (discriminator, count) in [
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE, 15u8),
        (raydium_cpmm::discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS, 16),
    ] {
        let mut keys: Vec<_> =
            (0..count).map(|_| Pubkey::new_unique().to_bytes().to_vec()).collect();
        keys.push(RAYDIUM_CPMM_PROGRAM_ID.to_bytes().to_vec());
        keys.push(Pubkey::new_unique().to_bytes().to_vec());
        let mut tx = SubscribeUpdateTransaction {
            slot: 123,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: vec![1; 64],
                transaction: Some(Transaction {
                    message: Some(Message {
                        account_keys: keys,
                        instructions: vec![
                            CompiledInstruction {
                                program_id_index: u32::from(count),
                                accounts: (0..count).collect(),
                                data: discriminator.to_vec(),
                            },
                            CompiledInstruction {
                                program_id_index: u32::from(count) + 1,
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                meta: Some(TransactionStatusMeta::default()),
                ..Default::default()
            }),
        };
        for parser in parsers {
            assert_eq!(parser(&tx, 0, None, Some(&filter)).len(), 1);
        }
        // A later instruction fails: the earlier collection is rolled back too.
        tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().err = Some(TransactionError {
            err: wincode::serialize(&solana_sdk::transaction::TransactionError::InstructionError(
                1,
                solana_sdk::instruction::InstructionError::Custom(6000),
            ))
            .unwrap(),
        });
        for parser in parsers {
            for filter in [None, Some(&filter)] {
                assert!(parser(&tx, 0, None, filter).is_empty());
            }
        }
    }
}

#[test]
fn failed_grpc_transaction_also_suppresses_previously_emitted_logs() {
    use prost::Message as _;
    use sol_parser_sdk::grpc::{
        parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency,
    };
    use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, TransactionError};
    let mut tx = SubscribeUpdateTransaction::decode(
        include_bytes!("fixtures/pumpfun_yellowstone_transaction.bin").as_slice(),
    )
    .unwrap();
    // Keep the recorded program logs, but remove instruction-derived events.
    tx.transaction
        .as_mut()
        .unwrap()
        .transaction
        .as_mut()
        .unwrap()
        .message
        .as_mut()
        .unwrap()
        .instructions
        .clear();
    let parsers =
        [parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency];
    for parser in parsers {
        assert!(!parser(&tx, 0, None, None).is_empty());
    }
    tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().err = Some(TransactionError {
        err: wincode::serialize(&solana_sdk::transaction::TransactionError::InstructionError(
            1,
            solana_sdk::instruction::InstructionError::Custom(6000),
        ))
        .unwrap(),
    });
    for parser in parsers {
        assert!(parser(&tx, 0, None, None).is_empty());
    }
}

#[test]
fn cpmm_swap_intents_require_all_fixed_accounts_and_arguments() {
    let keys: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
    for (disc, base_input) in [
        (raydium_cpmm::discriminators::SWAP_BASE_IN, true),
        (raydium_cpmm::discriminators::SWAP_BASE_OUT, false),
    ] {
        let mut data = disc.to_vec();
        data.extend_from_slice(&123u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        for count in 0..13 {
            assert!(raydium_cpmm::parse_instruction(
                &data,
                &keys[..count],
                Signature::default(),
                1,
                0,
                None,
            )
            .is_none());
        }
        for length in 0..24 {
            assert!(raydium_cpmm::parse_instruction(
                &data[..length],
                &keys,
                Signature::default(),
                1,
                0,
                None,
            )
            .is_none());
        }
        for count in 13..=16 {
            let DexEvent::RaydiumCpmmSwap(e) = raydium_cpmm::parse_instruction(
                &data,
                &keys[..count],
                Signature::default(),
                1,
                0,
                None,
            )
            .unwrap() else {
                panic!("swap")
            };
            assert_eq!(e.base_input, base_input);
            assert_eq!(e.pool_id, keys[3]);
            assert_eq!(e.observation_state, keys[12]);
        }
    }
}

#[test]
fn cpmm_lp_and_initialize_intents_reject_incomplete_wire_layouts() {
    let keys: Vec<_> = (0..22).map(|_| Pubkey::new_unique()).collect();
    for (disc, required, pool_index) in [
        (raydium_cpmm::discriminators::INITIALIZE, 20, 3),
        (raydium_cpmm::discriminators::DEPOSIT, 13, 2),
        (raydium_cpmm::discriminators::WITHDRAW, 14, 2),
    ] {
        let mut wire = disc.to_vec();
        for amount in [0u64, u64::MAX, 123] {
            wire.extend_from_slice(&amount.to_le_bytes());
        }
        let parse = |data: &[u8], accounts: &[Pubkey]| {
            raydium_cpmm::parse_instruction(data, accounts, Signature::default(), 1, 0, None)
        };
        for count in 0..required {
            assert!(parse(&wire, &keys[..count]).is_none(), "count={count}");
        }
        for length in 0..wire.len() {
            assert!(parse(&wire[..length], &keys).is_none(), "length={length}");
        }
        for count in required..=keys.len() {
            match parse(&wire, &keys[..count]).unwrap() {
                DexEvent::RaydiumCpmmInitialize(e) => {
                    assert_eq!(e.pool, keys[pool_index]);
                    assert_eq!((e.init_amount0, e.init_amount1), (0, u64::MAX));
                }
                DexEvent::RaydiumCpmmDeposit(e) => {
                    assert_eq!(e.pool, keys[pool_index]);
                    assert_eq!(
                        (e.lp_token_amount, e.token0_amount, e.token1_amount),
                        (0, u64::MAX, 123)
                    );
                }
                DexEvent::RaydiumCpmmWithdraw(e) => {
                    assert_eq!(e.pool, keys[pool_index]);
                    assert_eq!(
                        (e.lp_token_amount, e.token0_amount, e.token1_amount),
                        (0, u64::MAX, 123)
                    );
                }
                _ => panic!("unexpected event"),
            }
        }
    }
}

#[test]
fn cpmm_account_booleans_reject_invalid_wire_values() {
    use sol_parser_sdk::accounts::raydium_cpmm::{parse_amm_config, parse_pool_state};
    for (size, discriminator, offset, parser) in [
        (
            236,
            discriminators::AMM_CONFIG,
            9,
            parse_amm_config as fn(&AccountData, EventMetadata) -> Option<DexEvent>,
        ),
        (
            637,
            discriminators::POOL_STATE,
            390,
            parse_pool_state as fn(&AccountData, EventMetadata) -> Option<DexEvent>,
        ),
    ] {
        let mut account = AccountData {
            pubkey: Pubkey::new_unique(),
            owner: RAYDIUM_CPMM_PROGRAM_ID,
            data: vec![0; size],
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        };
        account.data[..8].copy_from_slice(discriminator);
        for value in 0..=u8::MAX {
            account.data[offset] = value;
            assert_eq!(
                parser(&account, EventMetadata::default()).is_some(),
                value <= 1,
                "boolean byte {value} at offset {offset}"
            );
        }
    }
}

#[test]
fn grpc_collection_rejects_malformed_static_and_loaded_account_addresses() {
    use sol_parser_sdk::grpc::instruction_parser::parse_instructions_enhanced;
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, Message, Transaction, TransactionStatusMeta,
    };
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmCollectCreatorFee]);
    for source in 0..4 {
        let mut keys: Vec<_> = (0..15).map(|_| Pubkey::new_unique().to_bytes().to_vec()).collect();
        let mut indices: Vec<u8> = (0..15).collect();
        let mut meta = TransactionStatusMeta::default();
        if source >= 2 {
            let loaded = keys.remove(3);
            for index in &mut indices {
                *index = if *index == 3 {
                    15
                } else if *index > 3 {
                    *index - 1
                } else {
                    *index
                };
            }
            if source == 2 {
                meta.loaded_writable_addresses.push(loaded);
            } else {
                meta.loaded_readonly_addresses.push(loaded);
            }
        }
        let program_index = keys.len();
        keys.push(RAYDIUM_CPMM_PROGRAM_ID.to_bytes().to_vec());
        let baseline = Some(Transaction {
            message: Some(Message {
                account_keys: keys,
                instructions: vec![CompiledInstruction {
                    program_id_index: program_index as u32,
                    accounts: indices,
                    data: raydium_cpmm::discriminators::COLLECT_CREATOR_FEE.to_vec(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        let parse = |tx: &Option<Transaction>, meta: &TransactionStatusMeta| {
            parse_instructions_enhanced(
                meta,
                tx,
                Signature::default(),
                1,
                0,
                None,
                0,
                Some(&filter),
            )
        };
        assert_eq!(parse(&baseline, &meta).len(), 1);
        for length in [0, 31, 33, 64] {
            let mut tx = baseline.clone();
            let mut invalid_meta = meta.clone();
            match source {
                0 => {
                    tx.as_mut().unwrap().message.as_mut().unwrap().account_keys[3].resize(length, 0)
                }
                1 => tx.as_mut().unwrap().message.as_mut().unwrap().account_keys[program_index]
                    .resize(length, 0),
                2 => invalid_meta.loaded_writable_addresses[0].resize(length, 0),
                _ => invalid_meta.loaded_readonly_addresses[0].resize(length, 0),
            }
            assert!(parse(&tx, &invalid_meta).is_empty(), "source={source}, length={length}");
        }
    }
}
