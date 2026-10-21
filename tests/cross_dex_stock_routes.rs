use sol_parser_sdk::{
    analyze_rpc_transaction_routes, analyze_yellowstone_transaction_routes, convert_rpc_to_grpc,
    stock_prices::StockTokenCatalog, transaction_route::SwapProtocol,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

#[test]
fn signed_cross_dex_routes_preserve_each_protocol_and_rpc_grpc_agree() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/cross_dex_stock_routes_20261020.json"))
            .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let rpc: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(case["rpc"].clone()).unwrap();
        let wire = rpc.transaction.transaction.decode().unwrap();
        wire.verify_and_hash_message().unwrap();
        let route = analyze_rpc_transaction_routes(&rpc, &[]).unwrap();
        assert_eq!(route.signature.to_string(), case["signature"].as_str().unwrap());
        assert!(route.succeeded);
        let business: SwapProtocol =
            serde_json::from_value(case["business_protocol"].clone()).unwrap();
        let conversion: SwapProtocol =
            serde_json::from_value(case["conversion_protocol"].clone()).unwrap();
        let b = route.legs.iter().find(|l| l.protocol == business).expect("business leg missing");
        let c =
            route.legs.iter().find(|l| l.protocol == conversion).expect("conversion leg missing");
        assert!(b.input_account == c.output_account || b.output_account == c.input_account);
        assert!(b.input_mint.is_some() && b.output_mint.is_some());
        assert!(b.actual_input_amount.is_some_and(|v| v > 0));
        if business == SwapProtocol::LaunchLab {
            assert!(b.stonkfun_mode.is_some());
        }
        let (meta, tx) = convert_rpc_to_grpc(&rpc).unwrap();
        let grpc = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
        assert_eq!(serde_json::to_value(&route).unwrap(), serde_json::to_value(grpc).unwrap());
        let catalog = StockTokenCatalog::screened_defaults();
        // The captured Stonk/Meteora bridge was removed for reserve below $500.
        // Its raw CPI legs must still parse, but it is no longer a default route.
        let expected = usize::from(case["name"] != "stonk_meteora");
        assert_eq!(catalog.conversion_legs(&route).count(), expected);
        for leg in catalog.conversion_legs(&route) {
            assert_ne!(leg.protocol, business);
        }
        let mut mismatched = route.clone();
        mismatched.legs.iter_mut().find(|l| l.protocol == conversion).unwrap().program =
            solana_sdk::pubkey::Pubkey::new_unique();
        assert_eq!(catalog.conversion_legs(&mismatched).count(), 0);
        let mut mismatched = route.clone();
        mismatched.legs.iter_mut().find(|l| l.protocol == conversion).unwrap().input_mint =
            Some(solana_sdk::pubkey::Pubkey::new_unique());
        assert_eq!(catalog.conversion_legs(&mismatched).count(), 0);
        let mut failed = route.clone();
        failed.succeeded = false;
        assert_eq!(catalog.conversion_legs(&failed).count(), 0);
        // Shred has no CPI metadata. Keeping only ALT resolution must not recreate
        // the inner swaps that this test knows from the successful RPC capture.
        let shred_meta = yellowstone_grpc_proto::prelude::TransactionStatusMeta {
            loaded_writable_addresses: meta.loaded_writable_addresses,
            loaded_readonly_addresses: meta.loaded_readonly_addresses,
            ..Default::default()
        };
        let outer_only = analyze_yellowstone_transaction_routes(&tx, &shred_meta, &[]);
        assert!(outer_only.legs.iter().all(|l| l.position.inner_index.is_none()
            && l.actual_input_amount.is_none()
            && l.actual_output_amount.is_none()));
    }
}
