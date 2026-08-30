use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use sol_parser_sdk::core::events::{DexEvent, PumpFunTradeEvent};
use sol_parser_sdk::grpc::benchmark_dedupe_log_instruction_events;
use solana_sdk::pubkey::Pubkey;
use std::hint::black_box;

fn pumpfun_pair() -> (Vec<DexEvent>, Vec<DexEvent>) {
    let mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let associated_user = Pubkey::new_unique();
    let bonding_curve = Pubkey::new_unique();
    let log_trade = PumpFunTradeEvent {
        mint,
        user,
        associated_user,
        is_buy: true,
        sol_amount: 977_777_777,
        token_amount: 30_765_521_374_696,
        ..Default::default()
    };
    let instruction_trade = PumpFunTradeEvent {
        mint,
        user,
        associated_user,
        bonding_curve,
        is_buy: true,
        amount: 30_765_521_374_696,
        max_sol_cost: 1_000_000_000,
        ix_name: "buy".to_string(),
        ..Default::default()
    };

    (vec![DexEvent::PumpFunTrade(log_trade)], vec![DexEvent::PumpFunBuy(instruction_trade)])
}

fn benchmark(c: &mut Criterion) {
    let template = pumpfun_pair();
    c.bench_function("log_instr_dedup/pumpfun_1_log_1_ix", |b| {
        b.iter_batched(
            || template.clone(),
            |(log_events, instr_events)| {
                black_box(benchmark_dedupe_log_instruction_events(log_events, instr_events));
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
