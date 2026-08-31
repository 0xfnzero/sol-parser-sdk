use criterion::{criterion_group, criterion_main, Criterion};
use sol_parser_sdk::core::events::{BlockMetaEvent, DexEvent, EventMetadata};
use sol_parser_sdk::grpc::buffers::MicroBatchBuffer;
use std::hint::black_box;

fn event() -> DexEvent {
    DexEvent::BlockMeta(BlockMetaEvent { metadata: EventMetadata::default() })
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("micro_batch_buffer/single_event_flush", |b| {
        let mut buffer = MicroBatchBuffer::new();
        b.iter(|| {
            black_box(buffer.push(1, 0, event(), 1, 100));
            black_box(buffer.flush());
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
