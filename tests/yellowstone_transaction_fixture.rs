use base64::{engine::general_purpose::STANDARD, Engine as _};
use prost::Message;
use sol_parser_sdk::grpc::{
    instruction_parser::parse_instructions_enhanced, parse_subscribe_update_transaction,
    parse_subscribe_update_transaction_low_latency, try_yellowstone_signature,
    yellowstone_message_version, EventType, EventTypeFilter, YellowstoneMessageVersion,
};
use sol_parser_sdk::{parse_yellowstone_transaction_cost, DexEvent};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, TransactionConfig};

const FIXTURE: &[u8] = include_bytes!("fixtures/pumpfun_yellowstone_transaction.bin");

fn fixture() -> SubscribeUpdateTransaction {
    SubscribeUpdateTransaction::decode(FIXTURE).expect("valid Yellowstone fixture")
}

fn pumpfun_filter() -> EventTypeFilter {
    EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpFunBuyExactSolIn,
    ])
}

#[test]
fn unscoped_truncated_create_log_does_not_mark_an_earlier_trade() {
    let mut transaction = fixture();
    let logs = &mut transaction
        .transaction
        .as_mut()
        .expect("transaction info")
        .meta
        .as_mut()
        .expect("transaction metadata")
        .log_messages;
    let mut create_data = vec![27, 114, 169, 77, 222, 235, 99, 118];
    create_data.push(0);
    logs.push(format!("Program data: {}", STANDARD.encode(create_data)));

    let filter = pumpfun_filter();
    let events =
        parse_subscribe_update_transaction_low_latency(&transaction, 0, None, Some(&filter));
    let parallel = parse_subscribe_update_transaction(&transaction, 0, None, Some(&filter));
    assert_eq!(
        serde_json::to_value(&events).expect("serialize low-latency events"),
        serde_json::to_value(&parallel).expect("serialize parallel events")
    );
    let trade = events.iter().find_map(|event| match event {
        DexEvent::PumpFunTrade(trade)
        | DexEvent::PumpFunBuy(trade)
        | DexEvent::PumpFunSell(trade)
        | DexEvent::PumpFunBuyExactSolIn(trade) => Some(trade),
        _ => None,
    });

    assert!(!trade.expect("PumpFun trade").is_created_buy);
}

fn remap_v0_index_for_v1(
    index: usize,
    static_len: usize,
    static_readonly_start: usize,
    loaded_writable_len: usize,
) -> usize {
    if index < static_readonly_start {
        index
    } else if index < static_len {
        index + loaded_writable_len
    } else if index < static_len + loaded_writable_len {
        static_readonly_start + index - static_len
    } else {
        index
    }
}

fn reorder_account_values_for_v1<T: Clone>(
    values: &mut Vec<T>,
    static_len: usize,
    static_readonly_start: usize,
    loaded_writable_len: usize,
    loaded_readonly_len: usize,
) {
    if values.len() != static_len + loaded_writable_len + loaded_readonly_len {
        return;
    }
    let old = std::mem::take(values);
    values.extend_from_slice(&old[..static_readonly_start]);
    values.extend_from_slice(&old[static_len..static_len + loaded_writable_len]);
    values.extend_from_slice(&old[static_readonly_start..static_len]);
    values.extend_from_slice(&old[static_len + loaded_writable_len..]);
}

fn convert_normalized_fixture_to_v1(transaction: &mut SubscribeUpdateTransaction) {
    let info = transaction.transaction.as_mut().expect("transaction info");
    let meta = info.meta.as_mut().expect("transaction metadata");
    let loaded_writable = std::mem::take(&mut meta.loaded_writable_addresses);
    let loaded_readonly = std::mem::take(&mut meta.loaded_readonly_addresses);
    let message = info.transaction.as_mut().and_then(|tx| tx.message.as_mut()).expect("message");
    let header = message.header.as_mut().expect("message header");
    let static_len = message.account_keys.len();
    let static_readonly_len = header.num_readonly_unsigned_accounts as usize;
    let static_readonly_start = static_len.checked_sub(static_readonly_len).expect("valid header");
    let loaded_writable_len = loaded_writable.len();
    let loaded_readonly_len = loaded_readonly.len();

    let remap = |index: usize| {
        remap_v0_index_for_v1(index, static_len, static_readonly_start, loaded_writable_len)
    };
    for instruction in &mut message.instructions {
        instruction.program_id_index = remap(instruction.program_id_index as usize) as u32;
        for index in &mut instruction.accounts {
            *index = remap(*index as usize) as u8;
        }
    }
    for group in &mut meta.inner_instructions {
        for instruction in &mut group.instructions {
            instruction.program_id_index = remap(instruction.program_id_index as usize) as u32;
            for index in &mut instruction.accounts {
                *index = remap(*index as usize) as u8;
            }
        }
    }
    for balance in meta.pre_token_balances.iter_mut().chain(&mut meta.post_token_balances) {
        balance.account_index = remap(balance.account_index as usize) as u32;
    }
    reorder_account_values_for_v1(
        &mut meta.pre_balances,
        static_len,
        static_readonly_start,
        loaded_writable_len,
        loaded_readonly_len,
    );
    reorder_account_values_for_v1(
        &mut meta.post_balances,
        static_len,
        static_readonly_start,
        loaded_writable_len,
        loaded_readonly_len,
    );

    let old_keys = std::mem::take(&mut message.account_keys);
    message.account_keys.extend_from_slice(&old_keys[..static_readonly_start]);
    message.account_keys.extend(loaded_writable);
    message.account_keys.extend_from_slice(&old_keys[static_readonly_start..]);
    message.account_keys.extend(loaded_readonly);
    header.num_readonly_unsigned_accounts = header
        .num_readonly_unsigned_accounts
        .checked_add(loaded_readonly_len as u32)
        .expect("readonly account count");
    message.address_table_lookups.clear();
    message.versioned = true;
    message.config = Some(TransactionConfig {
        priority_fee: Some(4_321),
        compute_unit_limit: Some(234_567),
        loaded_accounts_data_size_limit: Some(1_048_576),
        heap_size: Some(65_536),
    });
}

#[test]
fn fixture_parser_paths_are_equivalent_and_balances_are_consistent() {
    let transaction = fixture();
    let filter = pumpfun_filter();
    let parallel = parse_subscribe_update_transaction(&transaction, 0, None, Some(&filter));
    let sequential =
        parse_subscribe_update_transaction_low_latency(&transaction, 0, None, Some(&filter));

    assert_eq!(
        serde_json::to_value(&parallel).expect("serialize parallel events"),
        serde_json::to_value(&sequential).expect("serialize sequential events")
    );
    assert!(!sequential.is_empty());
    let info = transaction.transaction.as_ref().expect("transaction info");
    let meta = info.meta.as_ref().expect("transaction metadata");
    let instruction_events = parse_instructions_enhanced(
        meta,
        &info.transaction,
        try_yellowstone_signature(&info.signature).expect("fixture signature"),
        transaction.slot,
        info.index,
        None,
        0,
        Some(&filter),
    );
    assert!(!instruction_events.is_empty());
    assert!(
        instruction_events.iter().all(|event| event.metadata().recent_blockhash.is_some()),
        "public instruction parser must preserve recent_blockhash"
    );
    for event in &instruction_events {
        let trade = match event {
            DexEvent::PumpFunTrade(trade)
            | DexEvent::PumpFunBuy(trade)
            | DexEvent::PumpFunSell(trade)
            | DexEvent::PumpFunBuyExactSolIn(trade) => trade,
            other => panic!("unexpected instruction fixture event: {other:?}"),
        };
        trade.token_balance.expect("instruction parser final token balance");
        trade.sol_balance.expect("instruction parser final SOL balance");
    }

    for event in &sequential {
        let trade = match event {
            DexEvent::PumpFunBuy(trade)
            | DexEvent::PumpFunSell(trade)
            | DexEvent::PumpFunBuyExactSolIn(trade) => trade,
            other => panic!("unexpected fixture event: {other:?}"),
        };
        trade.token_balance.expect("final token balance");
        trade.sol_balance.expect("final SOL balance");
        assert!(trade.metadata.recent_blockhash.is_some());
    }
}

#[test]
fn malformed_signature_is_rejected_without_panicking() {
    let mut transaction = fixture();
    transaction.transaction.as_mut().expect("transaction info").signature.truncate(63);

    assert!(parse_subscribe_update_transaction(&transaction, 0, None, None).is_empty());
    assert!(parse_subscribe_update_transaction_low_latency(&transaction, 0, None, None).is_empty());
}

#[test]
fn normalized_v1_fixture_preserves_dex_events_and_exposes_config() {
    let mut transaction = fixture();
    let mut baseline = parse_subscribe_update_transaction_low_latency(
        &transaction,
        0,
        None,
        Some(&pumpfun_filter()),
    );
    validate_source_indices(&baseline, &transaction);
    convert_normalized_fixture_to_v1(&mut transaction);
    let encoded = transaction.encode_to_vec();
    let transaction = SubscribeUpdateTransaction::decode(encoded.as_slice())
        .expect("round-trip normalized V1 Yellowstone fixture");
    let info = transaction.transaction.as_ref().expect("transaction info");
    let message = info.transaction.as_ref().and_then(|tx| tx.message.as_ref()).expect("message");
    let meta = info.meta.as_ref().expect("transaction metadata");
    assert_eq!(yellowstone_message_version(message), YellowstoneMessageVersion::V1);
    assert!(message.address_table_lookups.is_empty());
    assert!(meta.loaded_writable_addresses.is_empty());
    assert!(meta.loaded_readonly_addresses.is_empty());
    let cost = parse_yellowstone_transaction_cost(
        info.transaction.as_ref().expect("normalized transaction"),
        meta,
    )
    .expect("V1 transaction cost");
    assert_eq!(cost.priority_fee_lamports, Some(4_321));
    assert_eq!(cost.compute_unit_limit, Some(234_567));
    assert_eq!(cost.loaded_accounts_data_size_limit, Some(1_048_576));
    assert_eq!(cost.heap_size, Some(65_536));

    let mut v1_events = parse_subscribe_update_transaction_low_latency(
        &transaction,
        0,
        None,
        Some(&pumpfun_filter()),
    );
    validate_source_indices(&v1_events, &transaction);
    // V1 reorders message keys, so raw indices legitimately differ. Resolved
    // addresses, instruction bytes and every trade field must still agree.
    for event in baseline.iter_mut().chain(&mut v1_events) {
        match event {
            DexEvent::PumpFunTrade(trade)
            | DexEvent::PumpFunBuy(trade)
            | DexEvent::PumpFunSell(trade)
            | DexEvent::PumpFunBuyExactSolIn(trade) => {
                if let Some(source) = &mut trade.third_program {
                    source.account_indices.clear();
                }
            }
            _ => {}
        }
    }
    assert_eq!(
        serde_json::to_value(v1_events).expect("serialize V1 events"),
        serde_json::to_value(baseline).expect("serialize baseline events")
    );
}

fn validate_source_indices(events: &[DexEvent], update: &SubscribeUpdateTransaction) {
    let info = update.transaction.as_ref().unwrap();
    let message = info.transaction.as_ref().unwrap().message.as_ref().unwrap();
    let meta = info.meta.as_ref().unwrap();
    let keys: Vec<_> = message
        .account_keys
        .iter()
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .map(|key| solana_sdk::pubkey::Pubkey::try_from(key.as_slice()).unwrap())
        .collect();
    assert!(events.iter().any(|event| event.third_program().is_some()));
    for source in events.iter().filter_map(DexEvent::third_program) {
        let ix = &message.instructions[source.instruction_index];
        assert_eq!(source.account_indices, ix.accounts);
        assert_eq!(source.program_id, keys[ix.program_id_index as usize]);
        assert_eq!(source.instruction_data, ix.data);
        assert_eq!(
            source.accounts,
            ix.accounts.iter().map(|i| Some(keys[*i as usize])).collect::<Vec<_>>()
        );
    }
}
