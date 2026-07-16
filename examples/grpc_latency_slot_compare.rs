use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

fn main() {
    let mut config = ClientConfig::low_latency();
    config.order_mode = OrderMode::MicroBatch;
    config.micro_batch_us = 50;

    println!("scenario=grpc_latency_slot_compare");
    println!("order_mode={:?}", config.order_mode);
    println!("micro_batch_us={}", config.micro_batch_us);
    println!("dry_run=true");
}
