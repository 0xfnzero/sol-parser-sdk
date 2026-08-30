use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::core::common_filler::fill_token_balances;
use sol_parser_sdk::core::events::{DexEvent, PumpFunTradeEvent};
use solana_sdk::pubkey::Pubkey;
use std::hint::black_box;
use yellowstone_grpc_proto::prelude::{
    Message, TokenBalance, Transaction, TransactionStatusMeta, UiTokenAmount,
};

const ACCOUNT_COUNT: usize = 32;
const USER_INDEX: usize = 13;
const ASSOCIATED_USER_INDEX: usize = 14;

fn token_balance(account_index: u32, amount: u64) -> TokenBalance {
    TokenBalance {
        account_index,
        ui_token_amount: Some(UiTokenAmount { amount: amount.to_string(), ..Default::default() }),
        ..Default::default()
    }
}

fn fixture() -> (DexEvent, Option<Transaction>, TransactionStatusMeta) {
    let mut accounts: Vec<Pubkey> = (0..ACCOUNT_COUNT).map(|_| Pubkey::new_unique()).collect();
    let user = Pubkey::new_unique();
    let associated_user = Pubkey::new_unique();
    accounts[USER_INDEX] = user;
    accounts[ASSOCIATED_USER_INDEX] = associated_user;

    let transaction = Some(Transaction {
        message: Some(Message {
            account_keys: accounts.iter().map(|key| key.to_bytes().to_vec()).collect(),
            ..Default::default()
        }),
        ..Default::default()
    });
    let meta = TransactionStatusMeta {
        pre_balances: vec![10_000_000_000; ACCOUNT_COUNT],
        post_balances: vec![9_000_000_000; ACCOUNT_COUNT],
        pre_token_balances: (7..=ASSOCIATED_USER_INDEX)
            .map(|index| token_balance(index as u32, 30_765_521_374_696 + index as u64))
            .collect(),
        post_token_balances: (7..=ASSOCIATED_USER_INDEX)
            .map(|index| token_balance(index as u32, 61_531_042_749_392 + index as u64))
            .collect(),
        ..Default::default()
    };
    let event = DexEvent::PumpFunBuy(PumpFunTradeEvent {
        user,
        associated_user,
        is_buy: true,
        ..Default::default()
    });

    (event, transaction, meta)
}

fn benchmark(c: &mut Criterion) {
    let (mut pumpfun_event, transaction, meta) = fixture();
    c.bench_function("pumpfun_balance_fill/32_accounts_8_token_balances", |b| {
        b.iter(|| {
            fill_token_balances(
                black_box(&mut pumpfun_event),
                black_box(&meta),
                black_box(&transaction),
            );
        })
    });

    let mut non_pumpfun_event = DexEvent::Error("not-pumpfun".to_string());
    c.bench_function("pumpfun_balance_fill/non_pumpfun_fast_return", |b| {
        b.iter(|| {
            fill_token_balances(
                black_box(&mut non_pumpfun_event),
                black_box(&meta),
                black_box(&transaction),
            );
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
