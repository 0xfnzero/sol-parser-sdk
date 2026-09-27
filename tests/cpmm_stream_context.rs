//! Synthetic invocation/IDL edge cases, NOT historical mainnet migration fixtures.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    core::events::*,
    grpc::{
        parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency,
        EventType, EventTypeFilter,
    },
    instr::{raydium_cpmm as cpmm, raydium_launchlab as launchlab},
    DexEvent,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, SubscribeUpdateTransaction,
    SubscribeUpdateTransactionInfo, Transaction, TransactionStatusMeta,
};

fn swap_accounts() -> Vec<Pubkey> {
    let mut a: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
    a[8] = spl_token::id();
    a[9] = spl_token_2022::id();
    a
}
fn instruction(exact_in: bool) -> Vec<u8> {
    let mut data = if exact_in {
        cpmm::discriminators::SWAP_BASE_IN
    } else {
        cpmm::discriminators::SWAP_BASE_OUT
    }
    .to_vec();
    data.extend(999_999_u64.to_le_bytes());
    data.extend(777_777_u64.to_le_bytes());
    data
}
fn swap_log(a: &[Pubkey], exact_in: bool, amount: u64, modern: bool) -> String {
    let mut data = sol_parser_sdk::logs::raydium_cpmm::discriminators::SWAP_EVENT.to_vec();
    data.extend(a[3].to_bytes());
    for value in [1000, 2000, amount, amount * 2, 3, 4] {
        data.extend(value.to_le_bytes());
    }
    data.push(u8::from(exact_in));
    if modern {
        data.extend(a[10].to_bytes());
        data.extend(a[11].to_bytes());
        data.extend(5_u64.to_le_bytes());
        data.extend(6_u64.to_le_bytes());
        data.push(1);
    }
    format!("Program data: {}", STANDARD.encode(data))
}
fn update(message: Message, meta: TransactionStatusMeta) -> SubscribeUpdateTransaction {
    SubscribeUpdateTransaction {
        slot: 42,
        transaction: Some(SubscribeUpdateTransactionInfo {
            signature: Signature::default().as_ref().to_vec(),
            transaction: Some(Transaction { message: Some(message), ..Default::default() }),
            meta: Some(meta),
            ..Default::default()
        }),
    }
}
fn swaps(events: Vec<DexEvent>) -> Vec<RaydiumCpmmSwapEvent> {
    events
        .into_iter()
        .filter_map(|e| if let DexEvent::RaydiumCpmmSwap(s) = e { Some(s) } else { None })
        .collect()
}
fn parse(tx: &SubscribeUpdateTransaction) -> Vec<RaydiumCpmmSwapEvent> {
    swaps(parse_subscribe_update_transaction_low_latency(tx, 0, None, None))
}
/// Outer aggregator with two inner swaps in the SAME pool, different actors and
/// opposite mint directions. Both use exact-input: mode cannot imply direction.
fn repeated_inner() -> (SubscribeUpdateTransaction, Vec<Pubkey>, Vec<Pubkey>) {
    let a = swap_accounts();
    let mut b = swap_accounts();
    b[3] = a[3];
    b[10] = a[11];
    b[11] = a[10];
    b[8] = a[9];
    b[9] = a[8];
    let router = Pubkey::new_unique();
    let keys = vec![router.to_bytes().to_vec(), cpmm::PROGRAM_ID_PUBKEY.to_bytes().to_vec()];
    let message = Message {
        account_keys: keys,
        instructions: vec![CompiledInstruction { program_id_index: 0, ..Default::default() }],
        ..Default::default()
    };
    let meta = TransactionStatusMeta {
        // Both writable and readonly loaded keys are exercised by the accounts.
        loaded_writable_addresses: a.iter().map(|p| p.to_bytes().to_vec()).collect(),
        loaded_readonly_addresses: b.iter().map(|p| p.to_bytes().to_vec()).collect(),
        inner_instructions: vec![InnerInstructions {
            index: 0,
            instructions: vec![
                InnerInstruction {
                    program_id_index: 1,
                    accounts: (2..15).collect(),
                    data: instruction(true),
                    stack_height: Some(2),
                },
                InnerInstruction {
                    program_id_index: 1,
                    accounts: (15..28).collect(),
                    data: instruction(true),
                    stack_height: Some(2),
                },
            ],
        }],
        log_messages: vec![
            format!("Program {router} invoke [1]"),
            format!("Program {} invoke [2]", cpmm::PROGRAM_ID_PUBKEY),
            swap_log(&a, true, 10, true),
            format!("Program {} success", cpmm::PROGRAM_ID_PUBKEY),
            format!("Program {} invoke [2]", cpmm::PROGRAM_ID_PUBKEY),
            swap_log(&b, true, 20, false),
            format!("Program {} success", cpmm::PROGRAM_ID_PUBKEY),
            format!("Program {router} success"),
        ],
        ..Default::default()
    };
    (update(message, meta), a, b)
}
#[test]
fn repeated_pool_distinct_actors_loaded_addresses_both_directions() {
    let (tx, a, b) = repeated_inner();
    for events in [
        parse_subscribe_update_transaction(&tx, 0, None, None),
        parse_subscribe_update_transaction_low_latency(&tx, 0, None, None),
    ] {
        let events = swaps(events);
        assert_eq!(events.len(), 2);
        for (i, (s, a)) in events.iter().zip([&a, &b]).enumerate() {
            let c = s.context.as_ref().unwrap();
            assert_eq!(c.payer, a[0]);
            assert_eq!(c.authority, a[1]);
            assert_eq!(c.amm_config, a[2]);
            assert_eq!(c.input_token_account, a[4]);
            assert_eq!(c.output_token_account, a[5]);
            assert_eq!(c.input_vault, a[6]);
            assert_eq!(c.output_vault, a[7]);
            assert_eq!(c.input_token_program, a[8]);
            assert_eq!(c.output_token_program, a[9]);
            assert_eq!(c.input_token_mint, a[10]);
            assert_eq!(c.output_token_mint, a[11]);
            assert_eq!(c.observation_state, a[12]);
            assert_eq!(
                s.instruction_index,
                Some(InstructionIndex { outer: 0, inner: Some(i as u32) })
            );
            assert_eq!(s.input_amount, if i == 0 { 10 } else { 20 });
            assert_eq!(s.amounts_source, CpmmSwapAmountsSource::SwapEvent);
            assert_eq!(s.transaction_success, Some(true));
            assert!(s.base_input);
        }
    }
}
#[test]
fn missing_first_event_does_not_shift_second_actor() {
    let (mut tx, a, b) = repeated_inner();
    tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages.remove(2);
    let s = parse(&tx);
    assert_eq!(s.len(), 2);
    let executed = s.iter().find(|s| s.input_amount == 20).unwrap();
    assert_eq!(executed.context.as_ref().unwrap().payer, b[0]);
    let fallback =
        s.iter().find(|s| s.amounts_source == CpmmSwapAmountsSource::InstructionOnly).unwrap();
    assert_eq!(fallback.input_amount, 0);
    assert_eq!(fallback.output_amount, 0);
    assert_eq!(fallback.context.as_ref().unwrap().payer, a[0]);
}
#[test]
fn incomplete_or_mismatched_trace_does_not_fill_log_actor() {
    for case in 0..5 {
        let (mut tx, _, _) = repeated_inner();
        let meta = tx.transaction.as_mut().unwrap().meta.as_mut().unwrap();
        match case {
            0 => {
                meta.log_messages.remove(1);
            }
            1 => {
                meta.log_messages.pop();
            }
            2 => {
                meta.inner_instructions[0].instructions[0].stack_height = None;
            }
            3 => {
                meta.log_messages[1] = format!("Program {} invoke [2]", Pubkey::new_unique());
            }
            _ => {
                meta.log_messages.push("Log truncated".into());
            }
        }
        for s in parse(&tx).iter().filter(|s| s.amounts_source == CpmmSwapAmountsSource::SwapEvent)
        {
            assert!(s.context.is_none(), "case {case}");
            assert!(s.instruction_index.is_none());
        }
    }
}
#[test]
fn mismatched_pool_mint_or_mode_does_not_bind_context() {
    for case in 0..4 {
        let (mut tx, mut a, _) = repeated_inner();
        if case == 0 {
            a[3] = Pubkey::new_unique();
        }
        if case == 1 {
            a[10] = Pubkey::new_unique();
        }
        if case == 2 {
            a[10] = Pubkey::default();
        }
        tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages[2] =
            swap_log(&a, case != 3, 10, true);
        let s = parse(&tx);
        let log = s.iter().find(|s| s.input_amount == 10).unwrap();
        assert!(log.context.is_none());
        assert!(log.instruction_index.is_none());
    }
}
#[test]
fn default_unknown_truncated_context_never_invents_accounts_or_amounts() {
    for case in 0..5 {
        let mut a = swap_accounts();
        match case {
            0 => a[0] = Pubkey::default(),
            1 => a[8] = Pubkey::new_unique(),
            2 => {
                a.truncate(4);
            }
            3 => a[11] = a[10],
            _ => a[5] = a[4],
        }
        let DexEvent::RaydiumCpmmSwap(s) =
            cpmm::parse_instruction(&instruction(false), &a, Signature::default(), 0, 0, None)
                .unwrap()
        else {
            panic!()
        };
        assert!(s.context.is_none());
        assert_eq!((s.input_amount, s.output_amount), (0, 0));
        assert!(!s.base_input);
        assert_eq!(s.amounts_source, CpmmSwapAmountsSource::InstructionOnly);
        assert_eq!(s.transaction_success, None);
    }
    assert!(cpmm::parse_instruction(
        &instruction(true)[..23],
        &swap_accounts(),
        Signature::default(),
        0,
        0,
        None
    )
    .is_none());
    assert!(cpmm::parse_instruction(
        &[255; 24],
        &swap_accounts(),
        Signature::default(),
        0,
        0,
        None
    )
    .is_none());
}
#[test]
fn legacy_prefix_and_partial_modern_tails_preserve_amounts_without_fabricating_context() {
    let a = swap_accounts();
    let log = swap_log(&a, true, 10, true);
    let bytes = STANDARD.decode(log.strip_prefix("Program data: ").unwrap()).unwrap();
    for len in [0, 80, 81, 82, 144, 145, 161, 162] {
        let event = sol_parser_sdk::logs::raydium_cpmm::parse_swap_event_from_data(
            &bytes[8..8 + len],
            EventMetadata::default(),
        );
        if len < 81 {
            assert!(event.is_none());
            continue;
        }
        let DexEvent::RaydiumCpmmSwap(s) = event.unwrap() else { panic!() };
        assert_eq!((s.input_amount, s.output_amount), (10, 20));
        assert!(s.context.is_none());
        assert_eq!(
            s.amounts_source,
            if len == 81 || len == 162 {
                CpmmSwapAmountsSource::SwapEvent
            } else {
                CpmmSwapAmountsSource::Unknown
            }
        );
    }
}
#[test]
fn duplicate_event_logs_in_one_invocation_are_not_trusted_as_two_fills() {
    let (mut tx, _, _) = repeated_inner();
    let logs = &mut tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages;
    logs.insert(3, logs[2].clone());
    let s = parse(&tx);
    for s in s.iter().filter(|s| s.input_amount == 10) {
        assert!(s.context.is_none());
    }
}
#[test]
fn caught_cpmm_cpi_failure_never_authorizes_an_executed_actor() {
    let (mut tx, _, _) = repeated_inner();
    let meta = tx.transaction.as_mut().unwrap().meta.as_mut().unwrap();
    meta.log_messages[3] =
        format!("Program {} failed: custom program error: 0x1", cpmm::PROGRAM_ID_PUBKEY);
    // Router catches the CPI error, so transaction-level success alone is insufficient.
    assert!(meta.err.is_none());
    let events = parse(&tx);
    let failed = events.iter().find(|s| s.input_amount == 10).unwrap();
    assert!(failed.context.is_none());
}

#[test]
fn old_serialized_swap_deserializes_with_unknown_context() {
    let s = RaydiumCpmmSwapEvent::default();
    let mut value = serde_json::to_value(s).unwrap();
    for name in
        ["context", "instruction_index", "amounts_source", "log_mints", "transaction_success"]
    {
        value.as_object_mut().unwrap().remove(name);
    }
    let s: RaydiumCpmmSwapEvent = serde_json::from_value(value).unwrap();
    assert!(s.context.is_none());
    assert!(s.instruction_index.is_none());
    assert_eq!(s.amounts_source, CpmmSwapAmountsSource::Unknown);
    assert_eq!(s.transaction_success, None);
}
#[test]
fn cpmm_event_filter_and_failed_status_are_explicit() {
    let (mut tx, _, _) = repeated_inner();
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]);
    assert_eq!(parse_subscribe_update_transaction(&tx, 0, None, Some(&filter)).len(), 2);
    let filter = EventTypeFilter::exclude_types(vec![EventType::RaydiumCpmmSwap]);
    assert!(parse_subscribe_update_transaction(&tx, 0, None, Some(&filter)).is_empty());
    tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().err = Some(Default::default());
    assert!(parse(&tx).iter().all(|s| s.transaction_success == Some(false)));
}
#[test]
fn ordinary_outer_and_inner_same_pool_swaps_are_not_merged_together() {
    let (mut tx, a, _) = repeated_inner();
    let info = tx.transaction.as_mut().unwrap();
    let message = info.transaction.as_mut().unwrap().message.as_mut().unwrap();
    message.instructions[0] = CompiledInstruction {
        program_id_index: 1,
        accounts: (2..15).collect(),
        data: instruction(true),
    };
    let logs = &mut info.meta.as_mut().unwrap().log_messages;
    logs[0] = format!("Program {} invoke [1]", cpmm::PROGRAM_ID_PUBKEY);
    *logs.last_mut().unwrap() = format!("Program {} success", cpmm::PROGRAM_ID_PUBKEY);
    logs.insert(1, swap_log(&a, true, 5, true));
    let s = parse(&tx);
    assert_eq!(s.len(), 3);
    assert_eq!(s.iter().map(|s| s.input_amount).collect::<Vec<_>>(), vec![5, 10, 20]);
}

fn migration_accounts() -> Vec<Pubkey> {
    // Account order and fixed addresses read from the checked-in IDL, not a chain capture.
    let idl: serde_json::Value =
        serde_json::from_str(include_str!("../idls/raydium_launchpad.json")).unwrap();
    let ix = idl["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "migrate_to_cpswap")
        .unwrap();
    let mut a: Vec<_> = ix["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            a["address"].as_str().map(|s| s.parse().unwrap()).unwrap_or_else(Pubkey::new_unique)
        })
        .collect();
    a[22] = spl_token_2022::id();
    a
}
fn migration_tx(a: &[Pubkey], inner: bool) -> SubscribeUpdateTransaction {
    let mut keys = vec![launchlab::PROGRAM_ID_PUBKEY.to_bytes().to_vec()];
    keys.extend(a.iter().map(|p| p.to_bytes().to_vec()));
    let ix = CompiledInstruction {
        program_id_index: 0,
        accounts: (1..29).collect(),
        data: launchlab::discriminators::MIGRATE_TO_CPSWAP.to_vec(),
    };
    if inner {
        keys.push(Pubkey::new_unique().to_bytes().to_vec());
        update(
            Message {
                account_keys: keys,
                instructions: vec![CompiledInstruction {
                    program_id_index: 29,
                    ..Default::default()
                }],
                ..Default::default()
            },
            TransactionStatusMeta {
                inner_instructions: vec![InnerInstructions {
                    index: 0,
                    instructions: vec![InnerInstruction {
                        program_id_index: ix.program_id_index,
                        accounts: ix.accounts,
                        data: ix.data,
                        stack_height: Some(2),
                    }],
                }],
                ..Default::default()
            },
        )
    } else {
        update(
            Message { account_keys: keys, instructions: vec![ix], ..Default::default() },
            TransactionStatusMeta::default(),
        )
    }
}
#[test]
fn current_idl_migration_flows_through_outer_inner_stream_and_filters() {
    for (platform, mode) in [
        (STONKFUN_STANDARD_PLATFORM_CONFIG, Some(StonkFunMode::Standard)),
        (STONKFUN_REWARD_PLATFORM_CONFIG, Some(StonkFunMode::Reward)),
        (Pubkey::new_unique(), None),
    ] {
        for inner in [false, true] {
            let mut a = migration_accounts();
            a[3] = platform;
            let tx = migration_tx(&a, inner);
            let filter =
                EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabMigrateCpmm]);
            for events in [
                parse_subscribe_update_transaction(&tx, 0, None, Some(&filter)),
                parse_subscribe_update_transaction_low_latency(&tx, 0, None, Some(&filter)),
            ] {
                assert_eq!(events.len(), 1);
                let DexEvent::RaydiumLaunchlabMigrateCpmm(m) = &events[0] else { panic!() };
                assert_eq!(m.old_pool, a[17]);
                assert_eq!(m.new_pool, a[5]);
                assert_eq!(m.payer, a[0]);
                assert_eq!(m.base_mint, a[1]);
                assert_eq!(m.quote_mint, a[2]);
                assert_eq!(m.platform_config, a[3]);
                assert_eq!(m.cpmm_program, cpmm::PROGRAM_ID_PUBKEY);
                assert_eq!(m.cpmm_config, a[10]);
                assert_eq!(m.cpmm_base_vault, a[8]);
                assert_eq!(m.cpmm_quote_vault, a[9]);
                assert_eq!(m.base_token_program, a[22]);
                assert_eq!(m.quote_token_program, a[23]);
                assert_eq!(m.stonkfun_mode(), mode);
                assert_eq!(m.transaction_success, Some(true));
                assert_eq!(
                    m.instruction_index,
                    Some(InstructionIndex { outer: 0, inner: inner.then_some(0) })
                );
            }
            let filter =
                EventTypeFilter::exclude_types(vec![EventType::RaydiumLaunchlabMigrateCpmm]);
            assert!(parse_subscribe_update_transaction(&tx, 0, None, Some(&filter)).is_empty());
            let filter = EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabMigrateAmm]);
            assert!(parse_subscribe_update_transaction(&tx, 0, None, Some(&filter)).is_empty());
        }
    }
}
#[test]
fn migration_success_requires_invocation_and_ancestor_success_not_just_meta() {
    let a = migration_accounts();
    for case in 0..4 {
        let mut tx = migration_tx(&a, true);
        let info = tx.transaction.as_mut().unwrap();
        let router = Pubkey::try_from(
            info.transaction.as_ref().unwrap().message.as_ref().unwrap().account_keys[29]
                .as_slice(),
        )
        .unwrap();
        let program = launchlab::PROGRAM_ID_PUBKEY;
        let meta = info.meta.as_mut().unwrap();
        meta.log_messages = vec![
            format!("Program {router} invoke [1]"),
            format!("Program {program} invoke [2]"),
            format!("Program {program} success"),
            format!("Program {router} success"),
        ];
        if case == 1 {
            meta.log_messages[2] = format!("Program {program} failed: custom program error: 0x1");
        }
        if case == 2 {
            meta.log_messages[3] = format!("Program {router} failed: custom program error: 0x1");
            meta.err = Some(Default::default());
        }
        if case == 3 {
            meta.log_messages.clear();
        }
        let events = parse_subscribe_update_transaction_low_latency(&tx, 0, None, None);
        assert_eq!(events.len(), 1);
        let DexEvent::RaydiumLaunchlabMigrateCpmm(m) = &events[0] else { panic!() };
        assert_eq!(
            m.invocation_success,
            match case {
                0 => Some(true),
                1 | 2 => Some(false),
                _ => None,
            }
        );
        assert_eq!(m.transaction_success, Some(case != 2));
    }
}

#[test]
fn exact_output_log_and_instruction_only_fallback_keep_mode_not_direction() {
    let (mut tx, a, b) = repeated_inner();
    let meta = tx.transaction.as_mut().unwrap().meta.as_mut().unwrap();
    for ix in &mut meta.inner_instructions[0].instructions {
        ix.data = instruction(false);
    }
    meta.log_messages[2] = swap_log(&a, false, 10, true);
    meta.log_messages[5] = swap_log(&b, false, 20, false);
    let events = parse(&tx);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|s| !s.base_input
        && s.context.is_some()
        && s.amounts_source == CpmmSwapAmountsSource::SwapEvent));
    tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages.clear();
    let events = parse(&tx);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|s| !s.base_input
        && s.input_amount == 0
        && s.output_amount == 0
        && s.amounts_source == CpmmSwapAmountsSource::InstructionOnly));
}

#[test]
fn migration_unknown_layout_wrong_program_and_default_accounts_fail_closed() {
    let original = migration_accounts();
    for case in 0..6 {
        let mut a = original.clone();
        match case {
            0 => {
                a.pop();
            }
            1 => a[4] = Pubkey::new_unique(),
            2 => a[17] = Pubkey::default(),
            3 => a[23] = spl_token_2022::id(),
            4 => a[22] = Pubkey::new_unique(),
            _ => a[5] = a[17],
        }
        assert!(launchlab::parse_instruction(
            &launchlab::discriminators::MIGRATE_TO_CPSWAP,
            &a,
            Signature::default(),
            0,
            0,
            None
        )
        .is_none());
    }
    for data in [
        launchlab::discriminators::MIGRATE_TO_CPSWAP[..7].to_vec(),
        [launchlab::discriminators::MIGRATE_TO_CPSWAP.as_slice(), &[0]].concat(),
        launchlab::discriminators::MIGRATE_TO_AMM.to_vec(),
    ] {
        assert!(launchlab::parse_instruction(&data, &original, Signature::default(), 0, 0, None)
            .is_none());
    }
    let mut tx = migration_tx(&original, false);
    tx.transaction
        .as_mut()
        .unwrap()
        .transaction
        .as_mut()
        .unwrap()
        .message
        .as_mut()
        .unwrap()
        .account_keys[0] = cpmm::PROGRAM_ID_PUBKEY.to_bytes().to_vec();
    assert!(parse_subscribe_update_transaction(&tx, 0, None, None).is_empty());
}
/// Two-pool route: each swap's flat account fields come from its own pool
/// invocation, and a swap log whose pool no CPMM invocation touches is never
/// filled from a sibling swap.
#[test]
fn flat_accounts_come_only_from_the_event_pool_invocation() {
    let (mut tx, a, mut b) = repeated_inner();
    b[3] = Pubkey::new_unique();
    let meta = tx.transaction.as_mut().unwrap().meta.as_mut().unwrap();
    meta.loaded_readonly_addresses = b.iter().map(|p| p.to_bytes().to_vec()).collect();
    meta.log_messages[5] = swap_log(&b, true, 20, false);
    for events in [
        parse_subscribe_update_transaction(&tx, 0, None, None),
        parse_subscribe_update_transaction_low_latency(&tx, 0, None, None),
    ] {
        let events = swaps(events);
        assert_eq!(events.len(), 2);
        for (s, a) in events.iter().zip([&a, &b]) {
            assert_eq!(s.pool_id, a[3]);
            assert_eq!(s.amm_config, a[2]);
            assert_eq!((s.input_vault, s.output_vault), (a[6], a[7]));
            assert_eq!((s.input_token_program, s.output_token_program), (a[8], a[9]));
            assert_eq!((s.input_token_mint, s.output_token_mint), (a[10], a[11]));
            assert_eq!(s.observation_state, a[12]);
        }
    }

    let (mut tx, _, _) = repeated_inner();
    let stray = swap_accounts();
    tx.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages[5] =
        swap_log(&stray, true, 20, false);
    let events = parse(&tx);
    let s = events.iter().find(|s| s.pool_id == stray[3]).unwrap();
    assert!(s.context.is_none());
    assert_eq!(s.amm_config, Pubkey::default());
    assert_eq!((s.input_vault, s.output_vault), (Pubkey::default(), Pubkey::default()));
    assert_eq!(s.input_token_mint, Pubkey::default());
    assert_eq!(s.observation_state, Pubkey::default());
}
