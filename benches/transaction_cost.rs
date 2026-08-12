use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::{
    parse_shred_transaction_cost,
    transaction_cost::{GLAIVE_TIP_ACCOUNTS, JITO_TIP_ACCOUNTS},
};
use solana_sdk::{
    hash::Hash,
    message::{compiled_instruction::CompiledInstruction, v0, MessageHeader, VersionedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use std::hint::black_box;

fn fixture(recipient: Pubkey) -> VersionedTransaction {
    let compute_budget = "ComputeBudget111111111111111111111111111111".parse().unwrap();
    let system_program = "11111111111111111111111111111111".parse().unwrap();
    let mut limit = vec![2];
    limit.extend_from_slice(&200_000u32.to_le_bytes());
    let mut price = vec![3];
    price.extend_from_slice(&5_000u64.to_le_bytes());
    let mut tip = 2u32.to_le_bytes().to_vec();
    tip.extend_from_slice(&10_000u64.to_le_bytes());

    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(v0::Message {
            header: MessageHeader::default(),
            account_keys: vec![Pubkey::new_unique(), compute_budget, system_program, recipient],
            recent_blockhash: Hash::default(),
            instructions: vec![
                CompiledInstruction::new_from_raw_parts(1, limit, vec![]),
                CompiledInstruction::new_from_raw_parts(1, price, vec![]),
                CompiledInstruction::new_from_raw_parts(2, tip, vec![0, 3]),
            ],
            address_table_lookups: vec![],
        }),
    }
}

fn transaction_cost(c: &mut Criterion) {
    let jito_transaction = fixture(JITO_TIP_ACCOUNTS[0]);
    let glaive_transaction = fixture(GLAIVE_TIP_ACCOUNTS[GLAIVE_TIP_ACCOUNTS.len() - 1]);
    let ordinary_transfer = fixture(Pubkey::new_unique());

    c.bench_function("transaction_cost/jito_tip", |b| {
        b.iter(|| parse_shred_transaction_cost(black_box(&jito_transaction)))
    });
    c.bench_function("transaction_cost/glaive_tip", |b| {
        b.iter(|| parse_shred_transaction_cost(black_box(&glaive_transaction)))
    });
    c.bench_function("transaction_cost/ordinary_transfer", |b| {
        b.iter(|| parse_shred_transaction_cost(black_box(&ordinary_transfer)))
    });
}

criterion_group!(benches, transaction_cost);
criterion_main!(benches);
