use prost14::Message;
use sol_parser_sdk::grpc::{
    instruction_parser::parse_instructions_enhanced, parse_subscribe_update_transaction,
    parse_subscribe_update_transaction_low_latency, try_yellowstone_signature, EventType,
    EventTypeFilter,
};
use sol_parser_sdk::DexEvent;
use yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction;

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

    for event in &sequential {
        let trade = match event {
            DexEvent::PumpFunBuy(trade)
            | DexEvent::PumpFunSell(trade)
            | DexEvent::PumpFunBuyExactSolIn(trade) => trade,
            other => panic!("unexpected fixture event: {other:?}"),
        };
        let pre_token = trade.pre_token_balance.expect("pre token balance");
        let post_token = trade.post_token_balance.expect("post token balance");
        trade.pre_sol_balance.expect("pre SOL balance");
        trade.post_sol_balance.expect("post SOL balance");
        let token_delta = if trade.is_buy {
            post_token.checked_sub(pre_token).expect("buy token delta")
        } else {
            pre_token.checked_sub(post_token).expect("sell token delta")
        };
        assert_eq!(token_delta, trade.token_amount);
        assert!(trade.metadata.recent_blockhash.is_some());
    }
}
