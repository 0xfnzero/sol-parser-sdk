use base64::{engine::general_purpose, Engine as _};
use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::{
    parse_rpc_transaction, parse_rpc_transaction_cost, parse_rpc_transaction_cost_with_signature,
    parse_rpc_transaction_with_cost,
};
use solana_sdk::{
    hash::Hash,
    message::{v1, MessageHeader, VersionedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta,
    EncodedTransaction, EncodedTransactionWithStatusMeta, TransactionBinaryEncoding,
    UiTransactionStatusMeta,
};
use std::hint::black_box;

fn fixture() -> EncodedConfirmedTransactionWithStatusMeta {
    let transaction = VersionedTransaction {
        signatures: vec![Signature::from([7; 64])],
        message: VersionedMessage::V1(v1::Message {
            header: MessageHeader { num_required_signatures: 1, ..Default::default() },
            config: v1::TransactionConfig::empty()
                .with_compute_unit_limit(200_000)
                .with_priority_fee(1_000),
            lifetime_specifier: Hash::new_unique(),
            account_keys: vec![Pubkey::new_unique()],
            instructions: Vec::new(),
        }),
    };
    let bytes = wincode::serialize(&transaction).expect("serialize V1 transaction");

    EncodedConfirmedTransactionWithStatusMeta {
        slot: 42,
        transaction: EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Binary(
                general_purpose::STANDARD.encode(bytes),
                TransactionBinaryEncoding::Base64,
            ),
            meta: Some(UiTransactionStatusMeta {
                err: None,
                status: Ok(()),
                fee: 6_000,
                pre_balances: vec![1_000_000],
                post_balances: vec![994_000],
                inner_instructions: OptionSerializer::None,
                log_messages: OptionSerializer::None,
                pre_token_balances: OptionSerializer::None,
                post_token_balances: OptionSerializer::None,
                rewards: OptionSerializer::None,
                loaded_addresses: OptionSerializer::None,
                return_data: OptionSerializer::None,
                compute_units_consumed: OptionSerializer::Some(10_000),
                cost_units: OptionSerializer::None,
            }),
            version: None,
        },
        block_time: Some(1_700_000_000),
        transaction_index: Some(0),
    }
}

fn benchmark(c: &mut Criterion) {
    let transaction = fixture();
    c.bench_function("rpc_transaction/v1_no_dex_events", |b| {
        b.iter(|| {
            black_box(
                parse_rpc_transaction(black_box(&transaction), None)
                    .expect("parse V1 RPC transaction"),
            );
        })
    });
    c.bench_function("rpc_transaction/cost_only_before_repeated_decode", |b| {
        b.iter(|| {
            let cost = parse_rpc_transaction_cost(black_box(&transaction))
                .expect("parse RPC transaction cost");
            let signature = transaction
                .transaction
                .transaction
                .decode()
                .and_then(|transaction| transaction.signatures.first().copied())
                .expect("decode signature");
            black_box((cost, signature));
        })
    });
    c.bench_function("rpc_transaction/cost_only_after_shared_decode", |b| {
        b.iter(|| {
            black_box(
                parse_rpc_transaction_cost_with_signature(black_box(&transaction))
                    .expect("parse RPC cost and signature"),
            );
        })
    });
    c.bench_function("rpc_transaction/events_cost_before_repeated_decode", |b| {
        b.iter(|| {
            let events = parse_rpc_transaction(black_box(&transaction), None)
                .expect("parse RPC transaction events");
            let cost = parse_rpc_transaction_cost(black_box(&transaction))
                .expect("parse RPC transaction cost");
            let signature = transaction
                .transaction
                .transaction
                .decode()
                .and_then(|transaction| transaction.signatures.first().copied())
                .expect("decode signature");
            black_box((events, cost, signature));
        })
    });
    c.bench_function("rpc_transaction/events_cost_after_shared_decode", |b| {
        b.iter(|| {
            black_box(
                parse_rpc_transaction_with_cost(black_box(&transaction), None)
                    .expect("parse RPC events and cost"),
            );
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
