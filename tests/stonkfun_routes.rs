use sol_parser_sdk::{
    analyze_rpc_transaction_routes, analyze_yellowstone_transaction_routes, convert_rpc_to_grpc,
    parse_rpc_transaction, DexEvent, SwapProtocol,
};
use solana_sdk::{pubkey, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

const GRADUATED: Pubkey = pubkey!("BUVzsLLLG7GWoyJVoU31pXiBveazA6GXTavZ9VD3CwS9");
fn fixture(name: &str) -> EncodedConfirmedTransactionWithStatusMeta {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}_rpc_transaction.json"));
    serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap()
}
#[test]
fn real_stonkfun_routes_preserve_order_mints_and_rpc_yellowstone_parity() {
    for name in ["stonkfun_reward", "stonkfun_graduated", "stonkfun_clmm", "stonkfun_complex"] {
        let tx = fixture(name);
        let route = analyze_rpc_transaction_routes(&tx, &[GRADUATED]).unwrap();
        assert!(route.succeeded);
        assert!(!route.legs.is_empty());
        assert!(
            route.legs.iter().all(|leg| leg.input_mint.is_some() && leg.output_mint.is_some()),
            "{name}"
        );
        assert!(route.legs.windows(2).all(|pair| (
            pair[0].position.outer_index,
            pair[0].position.inner_index
        ) < (
            pair[1].position.outer_index,
            pair[1].position.inner_index
        )));
        let (meta, transaction) = convert_rpc_to_grpc(&tx).unwrap();
        let grpc = analyze_yellowstone_transaction_routes(&transaction, &meta, &[GRADUATED]);
        assert_eq!(serde_json::to_string(&route).unwrap(), serde_json::to_string(&grpc).unwrap());
        assert!(parse_rpc_transaction(&tx, None).unwrap().iter().any(|e| matches!(
            e,
            DexEvent::RaydiumLaunchlabTrade(_) | DexEvent::RaydiumCpmmSwap(_)
        )));
    }
}
#[test]
fn graduated_attribution_requires_verified_pool_identity() {
    let tx = fixture("stonkfun_graduated");
    let unknown = analyze_rpc_transaction_routes(&tx, &[]).unwrap();
    assert!(unknown.legs.iter().all(|l| !l.stonkfun_graduated));
    let route = analyze_rpc_transaction_routes(&tx, &[GRADUATED]).unwrap();
    let attributed: Vec<_> = route.legs.iter().filter(|l| l.stonkfun_graduated).collect();
    assert_eq!(attributed.len(), 1);
    assert_eq!(attributed[0].pool, GRADUATED);
    assert_eq!(attributed[0].protocol, SwapProtocol::RaydiumCpmm);
    // Swaps never establish migration provenance merely by sharing quote mints.
    let mut registry = sol_parser_sdk::StonkFunPoolRegistry::default();
    assert_eq!(registry.observe_rpc_transaction(&tx).unwrap(), 0);
    assert_eq!(registry.observe_rpc_transaction(&fixture("stonkfun_failed")).unwrap(), 0);
    assert!(registry.verified_cpmm_pools().is_empty());
}
#[test]
fn failed_transaction_keeps_limits_but_never_reports_executed_swap_amounts() {
    let route = analyze_rpc_transaction_routes(&fixture("stonkfun_failed"), &[GRADUATED]).unwrap();
    assert!(!route.succeeded);
    assert!(!route.legs.is_empty());
    assert!(route
        .legs
        .iter()
        .all(|l| l.actual_input_amount.is_none() && l.actual_output_amount.is_none()));
}
#[test]
fn complex_routes_keep_all_branches_and_opaque_liquidity_programs() {
    let route = analyze_rpc_transaction_routes(&fixture("stonkfun_complex"), &[GRADUATED]).unwrap();
    assert_eq!(route.legs.len(), 5);
    assert_eq!(
        route.legs.iter().map(|l| l.protocol).collect::<Vec<_>>(),
        vec![
            SwapProtocol::RaydiumClmm,
            SwapProtocol::MeteoraDlmm,
            SwapProtocol::OrcaWhirlpool,
            SwapProtocol::RaydiumCpmm,
            SwapProtocol::MeteoraDlmm
        ]
    );
    assert!(route.unknown_invocations.iter().any(|i| i.has_token_transfers));
    assert!(route.legs.iter().all(|l| l.actual_input_amount.is_some()));
}
#[test]
fn migration_exposes_old_new_pools_and_platform_without_inventing_liquidity() {
    use sol_parser_sdk::instr::raydium_launchlab::{discriminators, parse_instruction};
    let mut accounts: Vec<_> = (0..28).map(|_| Pubkey::new_unique()).collect();
    accounts[3] = sol_parser_sdk::core::events::STONKFUN_REWARD_PLATFORM_CONFIG;
    accounts[4] = sol_parser_sdk::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID;
    let event = parse_instruction(
        &discriminators::MIGRATE_TO_CPSWAP,
        &accounts,
        Signature::default(),
        1,
        0,
        None,
    )
    .unwrap();
    let DexEvent::RaydiumLaunchlabMigrateAmm(event) = event else { panic!("migration") };
    assert_eq!(event.old_pool, accounts[17]);
    assert_eq!(event.new_pool, accounts[5]);
    assert_eq!(event.base_mint, accounts[1]);
    assert_eq!(event.quote_mint, accounts[2]);
    assert_eq!(event.stonkfun_mode(), Some(sol_parser_sdk::core::events::StonkFunMode::Reward));
    assert!(!event.liquidity_amount_known);
    assert!(parse_instruction(
        &discriminators::MIGRATE_TO_CPSWAP,
        &accounts[..17],
        Signature::default(),
        1,
        0,
        None
    )
    .is_none());
}
