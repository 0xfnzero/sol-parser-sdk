use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::grpc::instruction_parser::parse_instructions_enhanced;
use sol_parser_sdk::grpc::program_ids::PUMPFUN_PROGRAM;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::hint::black_box;
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, Message, Transaction, TransactionStatusMeta,
};

const ACCOUNT_COUNT: usize = 32;
const INSTRUCTION_COUNT: usize = 8;

fn fixture(
    track_program_context: bool,
) -> (Option<Transaction>, TransactionStatusMeta, EventTypeFilter) {
    let accounts: Vec<Pubkey> = (0..ACCOUNT_COUNT).map(|_| Pubkey::new_unique()).collect();
    let unsupported_program_index = ACCOUNT_COUNT - 1;
    let mut account_keys: Vec<Vec<u8>> =
        accounts.iter().map(|key| key.to_bytes().to_vec()).collect();
    if track_program_context {
        account_keys[unsupported_program_index] = PUMPFUN_PROGRAM.to_bytes().to_vec();
    }
    let instructions = (0..INSTRUCTION_COUNT)
        .map(|_| CompiledInstruction {
            program_id_index: unsupported_program_index as u32,
            accounts: (0..10).map(|index| index as u8).collect(),
            data: vec![0xAB; 16],
        })
        .collect();
    let transaction = Some(Transaction {
        message: Some(Message { account_keys, instructions, ..Default::default() }),
        ..Default::default()
    });

    (
        transaction,
        TransactionStatusMeta::default(),
        EventTypeFilter::include_only(vec![EventType::PumpFunTrade]),
    )
}

fn benchmark(c: &mut Criterion) {
    let (transaction, meta, filter) = fixture(false);
    c.bench_function("instruction_reject/8_outer_10_accounts", |b| {
        b.iter(|| {
            black_box(parse_instructions_enhanced(
                black_box(&meta),
                black_box(&transaction),
                Signature::default(),
                1,
                0,
                None,
                0,
                Some(black_box(&filter)),
            ));
        })
    });

    let (transaction, meta, filter) = fixture(true);
    c.bench_function("instruction_reject/8_context_program_unknown_ix", |b| {
        b.iter(|| {
            black_box(parse_instructions_enhanced(
                black_box(&meta),
                black_box(&transaction),
                Signature::default(),
                1,
                0,
                None,
                0,
                Some(black_box(&filter)),
            ));
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
