use sol_parser_sdk::{
    convert_rpc_to_grpc, parse_rpc_transaction, parse_rpc_transaction_cost,
    parse_rpc_transaction_cost_with_signature, parse_rpc_transaction_with_cost,
    parse_yellowstone_transaction_cost, DexEvent, ParseError,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use solana_transaction_status::{option_serializer::OptionSerializer, UiInstruction};

const FIXTURE: &str = include_str!("fixtures/pumpfun_rpc_transaction.json");
const PUMPSWAP_FIXTURE: &str = include_str!("fixtures/pumpswap_rpc_transaction.json");
const RAYDIUM_CPMM_FIXTURE: &str = include_str!("fixtures/raydium_cpmm_rpc_transaction.json");
const METEORA_DLMM_ORCA_FIXTURE: &str =
    include_str!("fixtures/meteora_dlmm_orca_rpc_transaction.json");
const SIGNATURE: &str =
    "QUYUtVPVkkjV2GGFTC4MfxtauNpRvWDViGCGxTckxPWbPZvEY92d1sZD9Lq5iK31sy3Drwy28gHmV89iRt9hz9R";

#[test]
fn parses_saved_mainnet_pumpfun_rpc_transaction() {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(FIXTURE).expect("valid RPC transaction fixture");
    let events = parse_rpc_transaction(&transaction, None).expect("parse RPC fixture");
    let trade = events.iter().find_map(|event| match event {
        DexEvent::PumpFunBuy(trade) => Some(trade),
        _ => None,
    });
    let trade = trade.expect("PumpFun buy");

    assert_eq!(trade.metadata.signature.to_string(), SIGNATURE);
    assert_eq!(trade.metadata.slot, 438_880_952);
    assert_eq!(trade.sol_amount, 977_777_777);
    assert_eq!(trade.token_amount, 30_765_521_374_696);
    assert_eq!(trade.ix_name, "buy");
    assert_eq!(trade.token_balance, Some(30_765_521_374_696));
    assert_eq!(trade.sol_balance, Some(21_071_753_199));
}

#[test]
fn optimized_rpc_cost_paths_match_full_conversion() {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(FIXTURE).expect("valid RPC transaction fixture");
    let (meta, grpc_transaction) =
        convert_rpc_to_grpc(&transaction).expect("convert full RPC transaction");
    let expected = parse_yellowstone_transaction_cost(&grpc_transaction, &meta)
        .expect("parse cost from full conversion");

    assert_eq!(
        parse_rpc_transaction_cost(&transaction).expect("parse optimized RPC cost"),
        expected
    );

    let (cost, signature) =
        parse_rpc_transaction_cost_with_signature(&transaction).expect("parse optimized RPC cost");
    assert_eq!(cost, expected);
    assert_eq!(signature.to_string(), SIGNATURE);

    let parsed =
        parse_rpc_transaction_with_cost(&transaction, None).expect("parse RPC events and cost");
    assert_eq!(parsed.cost, expected);
    assert_eq!(parsed.signature, signature);
    assert!(parsed.events.iter().any(|event| matches!(event, DexEvent::PumpFunBuy(_))));
}

fn parse_fixture(fixture: &str) -> Vec<DexEvent> {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(fixture).expect("valid RPC transaction fixture");
    parse_rpc_transaction(&transaction, None).expect("parse RPC fixture")
}

#[test]
fn base58_turbo_matches_bs58_for_saved_inner_instructions() {
    for fixture in [FIXTURE, PUMPSWAP_FIXTURE, RAYDIUM_CPMM_FIXTURE, METEORA_DLMM_ORCA_FIXTURE] {
        let transaction: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_str(fixture).expect("valid RPC transaction fixture");
        let meta = transaction.transaction.meta.as_ref().expect("transaction metadata");
        let OptionSerializer::Some(groups) = &meta.inner_instructions else {
            continue;
        };
        for instruction in groups.iter().flat_map(|group| &group.instructions) {
            let UiInstruction::Compiled(instruction) = instruction else {
                continue;
            };
            assert_eq!(
                base58_turbo::BITCOIN
                    .decode(&instruction.data)
                    .expect("base58-turbo decodes instruction"),
                bs58::decode(&instruction.data).into_vec().expect("bs58 decodes instruction")
            );
        }
    }
}

#[test]
fn base58_turbo_round_trips_solana_packet_sized_payloads() {
    for len in (0..=128).chain((160..=1_216).step_by(32)).chain([1_232]) {
        let mut state = 0x9e37_79b9_u32 ^ len as u32;
        let mut payload = vec![0u8; len];
        for byte in &mut payload {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        let leading_zeroes = len.min(17);
        payload[..leading_zeroes].fill(0);

        let encoded = bs58::encode(&payload).into_string();
        assert_eq!(
            base58_turbo::BITCOIN.decode(&encoded).expect("base58-turbo decodes generated payload"),
            payload,
            "payload length {len}"
        );
    }
}

#[test]
fn invalid_inner_instruction_base58_returns_a_conversion_error() {
    let mut transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(FIXTURE).expect("valid RPC transaction fixture");
    let meta = transaction.transaction.meta.as_mut().expect("transaction metadata");
    let OptionSerializer::Some(groups) = &mut meta.inner_instructions else {
        panic!("fixture inner instructions");
    };
    let compiled = groups
        .iter_mut()
        .flat_map(|group| &mut group.instructions)
        .find_map(|instruction| match instruction {
            UiInstruction::Compiled(compiled) => Some(compiled),
            _ => None,
        })
        .expect("compiled inner instruction");
    compiled.data = "0".to_string();

    let error = convert_rpc_to_grpc(&transaction).expect_err("invalid base58 must fail");
    assert!(matches!(
        error,
        ParseError::ConversionError(message)
            if message.contains("Failed to decode instruction data")
    ));
}

#[test]
fn parses_saved_mainnet_pumpswap_rpc_transaction() {
    let events = parse_fixture(PUMPSWAP_FIXTURE);
    let trade = events.iter().find_map(|event| match event {
        DexEvent::PumpSwapBuy(trade) => Some(trade),
        _ => None,
    });
    let trade = trade.expect("PumpSwap buy");

    assert_eq!(
        trade.metadata.signature.to_string(),
        "2qgqjVi7XtBeSudSkZhbQrsdFNeMUB4jApLdcdihAwcWWb5SxQvQKjrfr2ZQ12TrR4BEwvaY7PiHLqE3uZD24iw7"
    );
    assert_eq!(trade.metadata.slot, 438_881_023);
    assert_eq!(trade.base_amount_out, 7_317_003_080);
    assert_eq!(trade.quote_amount_in, 10_000_000);
    assert_eq!(trade.ix_name, "buy_exact_quote_in");
}

#[test]
fn parses_saved_mainnet_raydium_cpmm_rpc_transaction() {
    let swaps: Vec<_> = parse_fixture(RAYDIUM_CPMM_FIXTURE)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::RaydiumCpmmSwap(swap) => Some(swap),
            _ => None,
        })
        .collect();

    assert_eq!(swaps.len(), 3);
    assert_eq!(swaps[0].metadata.slot, 438_881_024);
    assert_eq!((swaps[0].input_amount, swaps[0].output_amount), (851_111, 3_788_666));
    assert_eq!((swaps[1].input_amount, swaps[1].output_amount), (636_739, 1_163_813_842));
    assert_eq!((swaps[2].input_amount, swaps[2].output_amount), (1_163_813_842, 2_843_080));
}

#[test]
fn parses_saved_mainnet_meteora_dlmm_and_nested_orca_rpc_transaction() {
    let events = parse_fixture(METEORA_DLMM_ORCA_FIXTURE);
    let dlmm = events.iter().find_map(|event| match event {
        DexEvent::MeteoraDlmmSwap(swap) => Some(swap),
        _ => None,
    });
    let dlmm = dlmm.expect("Meteora DLMM swap");
    assert_eq!(dlmm.metadata.slot, 438_873_646);
    assert_eq!(dlmm.amount_in, 2_738_183_783);
    assert_eq!(dlmm.amount_out, 81_555_062);
    assert_eq!(dlmm.mm_fee, 18_486_656);
    assert_eq!(dlmm.fee, 20_540_728);
    assert_eq!(dlmm.protocol_fee, 2_054_072);

    let orca = events.iter().find_map(|event| match event {
        DexEvent::OrcaWhirlpoolSwap(swap) => Some(swap),
        _ => None,
    });
    let orca = orca.expect("nested Orca Whirlpool swap");
    assert_eq!(orca.input_amount, 2_397_194_654);
    assert_eq!(orca.output_amount, 942_951_733);
}
