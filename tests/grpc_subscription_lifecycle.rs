//! Local Yellowstone transport regression. No Solana RPC or external endpoint.
use futures::{Stream, StreamExt};
use sol_parser_sdk::{
    grpc::{AccountFilter, ClientConfig, EventType, EventTypeFilter, YellowstoneGrpc},
    DexEvent,
};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};
use yellowstone_grpc_proto::geyser::geyser_server::{Geyser, GeyserServer};
use yellowstone_grpc_proto::prelude::*;

type Updates<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
#[derive(Clone)]
struct TestGeyser {
    connections: Arc<AtomicUsize>,
    requests: mpsc::UnboundedSender<(usize, SubscribeRequest)>,
}
#[tonic::async_trait]
impl Geyser for TestGeyser {
    type SubscribeStream = Updates<SubscribeUpdate>;
    async fn subscribe(
        &self,
        request: Request<tonic::Streaming<SubscribeRequest>>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let connection = self.connections.fetch_add(1, Ordering::SeqCst);
        let requests = self.requests.clone();
        let mut input = request.into_inner();
        let (output, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            let mut count = 0;
            while let Some(Ok(request)) = input.next().await {
                if request.ping.is_some() {
                    continue;
                }
                count += 1;
                let block_meta = !request.blocks_meta.is_empty();
                if requests.send((connection, request)).is_err() {
                    break;
                }
                if connection == 0 && count == 2 {
                    // Drop the first stream immediately after its dynamic filter update.
                    let _ = output.send(Err(Status::unavailable("test reconnect"))).await;
                    break;
                }
                if block_meta {
                    let _ = output
                        .send(Ok(SubscribeUpdate {
                            update_oneof: Some(subscribe_update::UpdateOneof::BlockMeta(
                                SubscribeUpdateBlockMeta {
                                    slot: 100 + connection as u64,
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        }))
                        .await;
                }
            }
        });
        Ok(Response::new(Box::pin(futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        }))))
    }
    type SubscribeDeshredStream = Updates<SubscribeUpdateDeshred>;
    async fn subscribe_deshred(
        &self,
        _: Request<tonic::Streaming<SubscribeDeshredRequest>>,
    ) -> Result<Response<Self::SubscribeDeshredStream>, Status> {
        Err(Status::unimplemented("unused"))
    }
    type SubscribeGossipStream = Updates<SubscribeUpdateGossip>;
    async fn subscribe_gossip(
        &self,
        _: Request<SubscribeGossipRequest>,
    ) -> Result<Response<Self::SubscribeGossipStream>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn subscribe_replay_info(
        &self,
        _: Request<SubscribeReplayInfoRequest>,
    ) -> Result<Response<SubscribeReplayInfoResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn ping(&self, _: Request<PingRequest>) -> Result<Response<PongResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get_latest_blockhash(
        &self,
        _: Request<GetLatestBlockhashRequest>,
    ) -> Result<Response<GetLatestBlockhashResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get_block_height(
        &self,
        _: Request<GetBlockHeightRequest>,
    ) -> Result<Response<GetBlockHeightResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get_slot(
        &self,
        _: Request<GetSlotRequest>,
    ) -> Result<Response<GetSlotResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn is_blockhash_valid(
        &self,
        _: Request<IsBlockhashValidRequest>,
    ) -> Result<Response<IsBlockhashValidResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get_version(
        &self,
        _: Request<GetVersionRequest>,
    ) -> Result<Response<GetVersionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tokio::test]
async fn dynamic_filters_and_block_progress_survive_real_grpc_reconnect() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let (shutdown, stopped) = oneshot::channel();
    let service = TestGeyser { connections: Arc::new(AtomicUsize::new(0)), requests };
    let incoming = futures::stream::unfold(listener, |listener| async {
        let result = listener.accept().await.map(|(socket, _)| socket);
        Some((result, listener))
    });
    let server = tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(GeyserServer::new(service))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = stopped.await;
            }),
    );
    let grpc = YellowstoneGrpc::new_with_config(
        format!("http://{address}"),
        None,
        ClientConfig {
            enable_tls: false,
            connection_timeout_ms: 1000,
            retry_delay_ms: 10,
            ..Default::default()
        },
    )
    .unwrap();
    let queue = grpc
        .subscribe_dex_events(
            vec![],
            vec![AccountFilter::new().add_account("old-account")],
            Some(EventTypeFilter::include_only(vec![
                EventType::AccountRawSnapshot,
                EventType::BlockMeta,
            ])),
        )
        .await
        .unwrap();
    let (first, initial) =
        tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
    assert_eq!(first, 0);
    assert_eq!(initial.blocks_meta.len(), 1);
    assert_eq!(initial.accounts["acc_0"].account, vec!["old-account"]);
    // Observing the request at the server precedes client subscription setup.
    // Wait for the first delivered block, not an arbitrary sleep.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(queue.pop(), Some(DexEvent::BlockMeta(e)) if e.metadata.slot == 100) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let initial_revision = grpc.subscription_status().continuity_revision;
    grpc.update_subscription(vec![], vec![AccountFilter::new().add_account("new-account")])
        .await
        .unwrap();
    let (same, updated) =
        tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
    assert_eq!(same, 0);
    assert_eq!(updated.blocks_meta.len(), 1, "dynamic updates must preserve block metadata");
    assert_eq!(updated.accounts["acc_0"].account, vec!["new-account"]);
    let (next, reconnected) =
        tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
    assert_eq!(next, 1);
    assert_eq!(
        reconnected.accounts["acc_0"].account,
        vec!["new-account"],
        "reconnect must not restore initial filters"
    );
    assert_eq!(reconnected.blocks_meta.len(), 1);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(queue.pop(),Some(DexEvent::BlockMeta(e)) if e.metadata.slot==101) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let status = grpc.subscription_status();
    assert!(status.connected && status.generation >= 2 && status.disconnects >= 1);
    assert!(status.continuity_revision > initial_revision);
    grpc.stop().await;
    assert!(!grpc.subscription_status().connected);
    let _ = shutdown.send(());
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap().unwrap();
}
