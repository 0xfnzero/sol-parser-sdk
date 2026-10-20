//! Replay saved execution metadata through the normal gRPC parser, without a network.
use sol_parser_sdk::{
    convert_rpc_to_grpc, grpc::parse_subscribe_update_transaction_low_latency, DexEvent,
};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

pub fn parse(
    rpc: serde_json::Value,
    received_us: i64,
) -> Result<Vec<DexEvent>, Box<dyn std::error::Error>> {
    let rpc = serde_json::from_value(rpc)?;
    let (meta, transaction) = convert_rpc_to_grpc(&rpc)?;
    let signature =
        transaction.signatures.first().ok_or("saved transaction has no signature")?.clone();
    let update = SubscribeUpdateTransaction {
        slot: rpc.slot,
        transaction: Some(SubscribeUpdateTransactionInfo {
            signature,
            transaction: Some(transaction),
            meta: Some(meta),
            ..Default::default()
        }),
    };
    Ok(parse_subscribe_update_transaction_low_latency(&update, received_us, None, None))
}
