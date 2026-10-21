//! Focused offline regression for wrapped Pump.fun provenance.
//! Signed transaction bytes are the only Shred input; saved execution metadata
//! is used for gRPC and to supply a preloaded ALT snapshot. No RPC or ABI inference.
use crate::{
    core::events::{DexEvent, RawInstructionPayload, TradeParseStatus},
    grpc::{EventType, EventTypeFilter},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use yellowstone_grpc_proto::prelude::{
    Message, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
};

fn assert_source(source: &RawInstructionPayload, message: &Message, keys: &[Pubkey]) {
    let outer = &message.instructions[source.instruction_index];
    assert_eq!(source.program_id, keys[outer.program_id_index as usize]);
    assert_eq!(source.instruction_data, outer.data);
    assert_eq!(source.account_indices, outer.accounts);
    assert_eq!(
        source.accounts,
        outer.accounts.iter().map(|i| Some(keys[*i as usize])).collect::<Vec<_>>()
    );
}

#[test]
fn reported_wrappers_preserve_third_program_in_grpc_and_shred() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/wrapped_pumpfun_reported.json"))
            .unwrap();
    let pump_filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
    for case in fixture["cases"].as_array().unwrap() {
        let rpc = serde_json::from_value(case["wire_rpc"].clone()).unwrap();
        let (meta, transaction) = crate::convert_rpc_to_grpc(&rpc).unwrap();
        let message = transaction.message.as_ref().unwrap();
        let addresses = |bytes: &[Vec<u8>]| {
            bytes.iter().map(|key| Pubkey::try_from(key.as_slice()).unwrap()).collect::<Vec<_>>()
        };
        let writable = addresses(&meta.loaded_writable_addresses);
        let readonly = addresses(&meta.loaded_readonly_addresses);
        let mut keys = addresses(&message.account_keys);
        keys.extend_from_slice(&writable);
        keys.extend_from_slice(&readonly);
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: transaction.signatures[0].clone(),
                transaction: Some(transaction.clone()),
                meta: Some(meta),
                index: 77,
                ..Default::default()
            }),
        };
        let grpc = crate::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            123456,
            None,
            Some(&pump_filter),
        );
        assert!(!grpc.is_empty(), "{}", case["signature"]);
        for event in &grpc {
            let DexEvent::PumpFunTrade(trade) = event else {
                panic!("unexpected event in PumpFunTrade subscription");
            };
            assert_source(
                trade.third_program.as_ref().expect("wrapped CPI source"),
                message,
                &keys,
            );
            assert_eq!(trade.parse_status, TradeParseStatus::ExecutionConfirmed);
            assert_eq!((trade.metadata.tx_index, trade.metadata.grpc_recv_us), (77, 123456));
        }

        let wire = STANDARD.decode(case["wire_rpc"]["transaction"][0].as_str().unwrap()).unwrap();
        let signed: VersionedTransaction = wincode::deserialize_exact(&wire).unwrap();
        signed.verify_and_hash_message().unwrap();
        assert_eq!(signed.signatures[0].to_string(), case["signature"].as_str().unwrap());
        let parse_shred = |filter: Option<&EventTypeFilter>| {
            let mut events = Vec::new();
            crate::shredstream::parse_transaction_dex_events_with_loaded_addresses(
                &signed,
                &writable,
                &readonly,
                signed.signatures[0],
                rpc.slot,
                77,
                123456,
                filter,
                &mut events,
            )
            .unwrap();
            events
        };
        let shred = parse_shred(Some(&pump_filter));
        assert!(!shred.is_empty(), "{}", case["signature"]);
        for event in &shred {
            let DexEvent::PumpFunTrade(trade) = event else { panic!("unexpected Shred event") };
            assert_source(trade.third_program.as_ref().unwrap(), message, &keys);
            assert_eq!(trade.parse_status, TradeParseStatus::Unresolved);
            assert_eq!((trade.sol_amount, trade.token_amount, trade.quote_amount), (0, 0, 0));
            assert!(trade.ix_name.is_empty());
            assert_eq!((trade.metadata.tx_index, trade.metadata.grpc_recv_us), (77, 123456));
            assert_eq!(
                trade.metadata.recent_blockhash.as_deref(),
                Some(signed.message.recent_blockhash().to_string().as_str())
            );
        }
        // Every executed wrapped Pump trade has its outer instruction represented,
        // but a candidate may only prepare accounts and never execute a trade.
        for trade in &grpc {
            let source = trade.third_program().unwrap();
            assert!(shred.iter().any(|event| event.third_program() == Some(source)));
        }
        assert_eq!(
            parse_shred(None).iter().filter(|event| event.third_program().is_some()).count(),
            shred.len()
        );
        for kind in [EventType::PumpFunBuy, EventType::PumpFunSell, EventType::RaydiumCpmmSwap] {
            assert!(parse_shred(Some(&EventTypeFilter::include_only(vec![kind]))).is_empty());
        }
        assert!(parse_shred(Some(&EventTypeFilter::exclude_types(vec![EventType::PumpFunTrade])))
            .iter()
            .all(|event| event.third_program().is_none()));
    }
}
