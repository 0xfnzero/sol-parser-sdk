use criterion::{criterion_group, criterion_main, Criterion};
use solana_entry::entry::Entry;
use solana_sdk::{
    hash::Hash,
    message::{legacy, MessageHeader, VersionedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use std::hint::black_box;

fn fixture() -> (Vec<Entry>, Vec<u8>) {
    let transaction = VersionedTransaction {
        signatures: vec![Signature::from([7; 64])],
        message: VersionedMessage::Legacy(legacy::Message {
            header: MessageHeader { num_required_signatures: 1, ..Default::default() },
            account_keys: vec![Pubkey::new_unique()],
            recent_blockhash: Hash::new_unique(),
            instructions: Vec::new(),
        }),
    };
    let entries = (0..32)
        .map(|index| Entry {
            num_hashes: index,
            hash: Hash::new_unique(),
            transactions: vec![transaction.clone(); 16],
        })
        .collect::<Vec<_>>();
    let bytes = wincode::serialize(&entries).expect("serialize Entry fixture");
    (entries, bytes)
}

fn benchmark(c: &mut Criterion) {
    let (expected, bytes) = fixture();
    let decoded: Vec<Entry> = wincode::deserialize(&bytes).expect("decode Entry fixture");
    assert_eq!(decoded, expected);

    c.bench_function("shred_entry_decode/wincode_32x16", |b| {
        b.iter(|| {
            let entries: Vec<Entry> =
                wincode::deserialize(black_box(&bytes)).expect("decode Entry fixture");
            black_box(entries);
        })
    });
    c.bench_function("shred_entry_decode/wincode_32x16_sanitized", |b| {
        b.iter(|| {
            let entries: Vec<Entry> =
                wincode::deserialize(black_box(&bytes)).expect("decode Entry fixture");
            for entry in &entries {
                for transaction in &entry.transactions {
                    black_box(transaction.sanitize()).expect("sanitize transaction");
                }
            }
            black_box(entries);
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
