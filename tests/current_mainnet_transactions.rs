use sol_parser_sdk::{parse_transaction_from_rpc, DexEvent};
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

fn run_mainnet_tests() -> bool {
    std::env::var("RUN_MAINNET_TESTS").as_deref() == Ok("1")
}

fn rpc_client() -> RpcClient {
    RpcClient::new(
        std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
    )
}

fn parse(signature: &str) -> Vec<DexEvent> {
    let signature = Signature::from_str(signature).expect("valid fixture signature");
    parse_transaction_from_rpc(&rpc_client(), &signature, None)
        .unwrap_or_else(|error| panic!("{signature}: {error}"))
}

// These transactions were captured from current mainnet traffic on 2026-08-13.
// Run with: RUN_MAINNET_TESTS=1 SOLANA_RPC_URL=<optional archive RPC> cargo test --test current_mainnet_transactions

#[test]
fn current_meteora_dlmm_swap_and_nested_orca() {
    if !run_mainnet_tests() {
        return;
    }
    const SIGNATURE: &str =
        "eEWaGsbRPoiD36Xf3epzSMmdtXX36va76b13YfsDV3ncsxQHBTjC68zZ8mbzFXTNWy3n3qKUAHjgHBconX4Gu1i";
    let events = parse(SIGNATURE);
    let swaps: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            DexEvent::MeteoraDlmmSwap(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(swaps.len(), 1);
    assert_eq!(swaps[0].metadata.signature.to_string(), SIGNATURE);
    assert_eq!(swaps[0].metadata.slot, 438_873_646);
    assert_eq!(swaps[0].amount_in, 2_738_183_783);
    assert_eq!(swaps[0].amount_out, 81_555_062);
    assert_eq!(swaps[0].fee, 18_486_656);
    assert_eq!(swaps[0].protocol_fee, 2_054_072);

    let orca: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            DexEvent::OrcaWhirlpoolSwap(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(orca.len(), 1);
    assert_eq!(orca[0].input_amount, 2_397_194_654);
    assert_eq!(orca[0].output_amount, 942_951_733);
}

#[test]
fn current_meteora_dlmm_add_liquidity_occurrences() {
    if !run_mainnet_tests() {
        return;
    }
    const SIGNATURE: &str =
        "h3sGiriW4jCGgbnNF8DaEKnsWhjtcH5ZM1dkyZFidgD2fx9aDNatray38yRmkxaWez3g5qFNpyXhE8ho716vjgp";
    let adds: Vec<_> = parse(SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::MeteoraDlmmAddLiquidity(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(adds.len(), 3);
    assert_eq!(adds[0].metadata.slot, 438_873_652);
    assert_eq!(adds[0].amounts, [0, 389_400_592]);
    assert_eq!(adds[1].amounts, [4_957_546_677, 65_633_812]);
    assert_eq!(adds[2].amounts, [6_984_260_460, 0]);
}

#[test]
fn current_pumpfun_and_pumpswap_trades() {
    if !run_mainnet_tests() {
        return;
    }
    const PUMPFUN_SIGNATURE: &str =
        "QUYUtVPVkkjV2GGFTC4MfxtauNpRvWDViGCGxTckxPWbPZvEY92d1sZD9Lq5iK31sy3Drwy28gHmV89iRt9hz9R";
    let pumpfun: Vec<_> = parse(PUMPFUN_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::PumpFunBuy(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(pumpfun.len(), 1);
    assert_eq!(pumpfun[0].metadata.slot, 438_880_952);
    assert_eq!(pumpfun[0].sol_amount, 977_777_777);
    assert_eq!(pumpfun[0].token_amount, 30_765_521_374_696);
    assert_eq!(pumpfun[0].ix_name, "buy");

    const PUMPSWAP_SIGNATURE: &str =
        "2qgqjVi7XtBeSudSkZhbQrsdFNeMUB4jApLdcdihAwcWWb5SxQvQKjrfr2ZQ12TrR4BEwvaY7PiHLqE3uZD24iw7";
    let pumpswap: Vec<_> = parse(PUMPSWAP_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::PumpSwapBuy(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(pumpswap.len(), 1);
    assert_eq!(pumpswap[0].metadata.slot, 438_881_023);
    assert_eq!(pumpswap[0].base_amount_out, 7_317_003_080);
    assert_eq!(pumpswap[0].quote_amount_in, 10_000_000);
    assert_eq!(pumpswap[0].ix_name, "buy_exact_quote_in");
}

#[test]
fn current_raydium_clmm_cpmm_amm_v4_and_launchlab() {
    if !run_mainnet_tests() {
        return;
    }
    const CLMM_SIGNATURE: &str =
        "2TyjCWrh3zqNmDg7NgdGAFqGaCbEkUHE8zrjzsRyqRVTXYZNKFFJgFEDce5mD4se3h2u6GyNJLXbeAW1ad8dsApq";
    let clmm: Vec<_> = parse(CLMM_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::RaydiumClmmSwap(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(clmm.len(), 1);
    assert_eq!(clmm[0].metadata.slot, 438_880_315);
    assert_eq!((clmm[0].amount_0, clmm[0].amount_1), (156_679, 11_888));

    const CPMM_SIGNATURE: &str =
        "4v27ccyrAgpCdCHLvjvn8smFn4Fb4HGcRVTSt952eNcF5jg5niA5bKRLPoGrzxXZdZULEZujgA5TXdESNbwmFYE8";
    let cpmm: Vec<_> = parse(CPMM_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::RaydiumCpmmSwap(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(cpmm.len(), 3);
    assert_eq!(cpmm[0].metadata.slot, 438_881_024);
    assert_eq!((cpmm[0].input_amount, cpmm[0].output_amount), (851_111, 3_788_666));
    assert_eq!((cpmm[1].input_amount, cpmm[1].output_amount), (636_739, 1_163_813_842));
    assert_eq!((cpmm[2].input_amount, cpmm[2].output_amount), (1_163_813_842, 2_843_080));

    const AMM_V4_SIGNATURE: &str =
        "2iHYs4AHC5nutcbBxpA5aptBYTGaDUYBgamohfetDnAPiPBW5NkguxgnjVF5886Jy8MZ19UXdeZyPKq9C5wqAki4";
    let amm_v4: Vec<_> = parse(AMM_V4_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::RaydiumAmmV4Swap(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(amm_v4.len(), 1);
    assert_eq!(amm_v4[0].metadata.slot, 438_881_026);
    assert_eq!(amm_v4[0].amount_in, 28_804_156_949_609);
    assert_eq!(amm_v4[0].amount_out, 428_715_251);

    const LAUNCHLAB_SIGNATURE: &str =
        "4pSXdZEdL3oFCcbccroG7GkV4oEVtbygS2pBVP28chfNETEN8yqE3q6gMws4F2ZsbfE8rDGEbgBqTnv5xahH5RFT";
    let launchlab: Vec<_> = parse(LAUNCHLAB_SIGNATURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::RaydiumLaunchlabTrade(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(launchlab.len(), 1);
    assert_eq!(launchlab[0].metadata.slot, 438_880_206);
    assert_eq!(launchlab[0].amount_in, 511_580_573);
    assert_eq!(launchlab[0].amount_out, 5_169_841_048_834);
}
