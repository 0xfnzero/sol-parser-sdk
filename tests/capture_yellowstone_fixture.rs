use futures_util::StreamExt;
use prost14::Message;
use sol_parser_sdk::grpc::{
    build_subscribe_request, connect_yellowstone_geyser, parse_subscribe_update_transaction,
    GeyserConnectConfig, Protocol, TransactionFilter,
};
use std::path::PathBuf;
use std::time::Duration;
use yellowstone_grpc_proto::prelude::{subscribe_update, SubscribeUpdateTransaction};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_live_pumpfun_yellowstone_transaction() {
    if std::env::var("CAPTURE_YELLOWSTONE_FIXTURE").as_deref() != Ok("1") {
        return;
    }

    let endpoint = std::env::var("GRPC_URL").expect("GRPC_URL must be set");
    let token = std::env::var("GRPC_TOKEN").expect("GRPC_TOKEN must be set");
    let mut client = connect_yellowstone_geyser(
        &endpoint,
        GeyserConnectConfig {
            connect_timeout: Duration::from_secs(10),
            x_token: Some(token),
            ..Default::default()
        },
    )
    .await
    .expect("gRPC connection should succeed");

    let request =
        build_subscribe_request(&[TransactionFilter::for_protocols(&[Protocol::PumpFun])], &[]);
    let mut stream = client.subscribe_once(request).await.expect("subscription should start");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);

    loop {
        let update = tokio::time::timeout_at(deadline, stream.next())
            .await
            .expect("timed out waiting for a PumpFun transaction")
            .expect("gRPC stream ended")
            .expect("gRPC update should decode");
        let Some(subscribe_update::UpdateOneof::Transaction(tx)) = update.update_oneof else {
            continue;
        };
        if parse_subscribe_update_transaction(&tx, 0, None, None).is_empty() {
            continue;
        }

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pumpfun_yellowstone_transaction.bin");
        std::fs::create_dir_all(fixture_path.parent().unwrap()).expect("fixture directory");
        std::fs::write(&fixture_path, tx.encode_to_vec()).expect("write fixture");

        let bytes = std::fs::read(&fixture_path).expect("read fixture back");
        let decoded = SubscribeUpdateTransaction::decode(bytes.as_slice()).expect("decode fixture");
        let events = parse_subscribe_update_transaction(&decoded, 0, None, None);
        assert!(!events.is_empty(), "captured fixture must remain parseable");
        return;
    }
}
