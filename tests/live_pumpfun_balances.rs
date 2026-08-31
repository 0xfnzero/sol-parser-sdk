use sol_parser_sdk::grpc::{
    ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol, TransactionFilter,
    YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_pumpfun_trade_has_consistent_balances() {
    if std::env::var("RUN_LIVE_GRPC_TEST").as_deref() != Ok("1") {
        return;
    }

    let endpoint = std::env::var("GRPC_URL").expect("GRPC_URL must be set");
    let token = std::env::var("GRPC_TOKEN").expect("GRPC_TOKEN must be set");
    let config = ClientConfig {
        enable_metrics: false,
        connection_timeout_ms: 10_000,
        request_timeout_ms: 30_000,
        enable_tls: endpoint.starts_with("https://"),
        order_mode: OrderMode::Unordered,
        ..Default::default()
    };
    let grpc = YellowstoneGrpc::new_with_config(endpoint, Some(token), config)
        .expect("gRPC client should be created");
    let protocols = vec![Protocol::PumpFun];
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpFunBuyExactSolIn,
    ]);
    let queue = grpc
        .subscribe_dex_events(
            vec![TransactionFilter::for_protocols(&protocols)],
            Vec::new(),
            Some(event_filter),
        )
        .await
        .expect("PumpFun subscription should start");

    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        if let Some(event) = queue.pop() {
            let trade = match &event {
                DexEvent::PumpFunBuy(trade)
                | DexEvent::PumpFunSell(trade)
                | DexEvent::PumpFunBuyExactSolIn(trade) => trade,
                _ => continue,
            };
            let pre_token = trade.pre_token_balance.expect("pre token balance should be present");
            let post_token =
                trade.post_token_balance.expect("post token balance should be present");
            let pre_sol = trade.pre_sol_balance.expect("pre SOL balance should be present");
            let post_sol = trade.post_sol_balance.expect("post SOL balance should be present");

            let token_delta = if trade.is_buy {
                assert!(post_token >= pre_token, "buy must not reduce the user's token balance");
                post_token - pre_token
            } else {
                assert!(pre_token >= post_token, "sell must not increase the user's token balance");
                pre_token - post_token
            };
            assert!(token_delta > 0, "trade must change the user's token balance");
            assert_eq!(
                token_delta, trade.token_amount,
                "token balance delta must equal the parsed trade amount"
            );
            println!(
                "verified {} {}: user {}, quote {}, event amount {}, token {} -> {}, SOL {} -> {} lamports",
                trade.metadata.signature,
                if trade.is_buy { "buy" } else { "sell" },
                trade.user,
                trade.quote_mint,
                trade.token_amount,
                pre_token,
                post_token,
                pre_sol,
                post_sol
            );
            return;
        }

        assert!(Instant::now() < deadline, "timed out waiting for a PumpFun trade");
        tokio::task::yield_now().await;
    }
}
