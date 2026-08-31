use criterion::{criterion_group, criterion_main, Criterion};
use prost::Message;
use sol_parser_sdk::grpc::{
    parse_subscribe_update_transaction_low_latency, EventType, EventTypeFilter,
};
use std::hint::black_box;
use yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction;

const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/pumpfun_yellowstone_transaction.bin");

fn benchmark(c: &mut Criterion) {
    let transaction =
        SubscribeUpdateTransaction::decode(FIXTURE).expect("valid Yellowstone fixture");
    let filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpFunBuyExactSolIn,
    ]);

    c.bench_function("yellowstone_transaction/pumpfun_low_latency", |b| {
        b.iter(|| {
            black_box(parse_subscribe_update_transaction_low_latency(
                black_box(&transaction),
                0,
                None,
                Some(black_box(&filter)),
            ));
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
