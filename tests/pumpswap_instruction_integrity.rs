use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_sdk::pubkey::Pubkey;
use sol_parser_sdk::{
    instr::{pump_amm, pump_amm_inner},
    logs::pump_amm as logs,
    DexEvent, EventMetadata,
};

fn parse(disc: [u8; 8], body: &[u8], keys: &[Pubkey]) -> Option<DexEvent> {
    let mut data = disc.to_vec();
    data.extend_from_slice(body);
    pump_amm::parse_instruction(&data, keys, Default::default(), 1, 0, None)
}

#[test]
fn liquidity_instructions_follow_idl_accounts_and_amounts() {
    let keys: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
    let amounts = [0u64, 123, u64::MAX];
    let body: Vec<_> = amounts.iter().flat_map(|n| n.to_le_bytes()).collect();
    for disc in [
        pump_amm::discriminators::DEPOSIT,
        pump_amm::discriminators::WITHDRAW,
    ] {
        for len in 0..24 {
            assert!(parse(disc, &body[..len], &keys).is_none(), "len={len}");
        }
        for count in 0..9 {
            assert!(parse(disc, &body, &keys[..count]).is_none());
        }
        let (pool, user, base, quote, lp, actual) = match parse(disc, &body, &keys).unwrap() {
            DexEvent::PumpSwapLiquidityAdded(e) => (
                e.pool,
                e.user,
                e.user_base_token_account,
                e.user_quote_token_account,
                e.user_pool_token_account,
                [
                    e.lp_token_amount_out,
                    e.max_base_amount_in,
                    e.max_quote_amount_in,
                ],
            ),
            DexEvent::PumpSwapLiquidityRemoved(e) => (
                e.pool,
                e.user,
                e.user_base_token_account,
                e.user_quote_token_account,
                e.user_pool_token_account,
                [
                    e.lp_token_amount_in,
                    e.min_base_amount_out,
                    e.min_quote_amount_out,
                ],
            ),
            _ => panic!("liquidity"),
        };
        assert_eq!(
            (pool, user, base, quote, lp),
            (keys[0], keys[2], keys[6], keys[7], keys[8])
        );
        assert_eq!(actual, amounts);
    }
}

#[test]
fn create_pool_rejects_partial_fields_and_invalid_bools() {
    let keys: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
    let mut body = vec![0; 62];
    body[..2].copy_from_slice(&42u16.to_le_bytes());
    body[2..10].copy_from_slice(&123u64.to_le_bytes());
    body[10..18].copy_from_slice(&456u64.to_le_bytes());
    body[18..50].copy_from_slice(keys[2].as_ref());
    body[52..60].copy_from_slice(&250u64.to_le_bytes());
    for count in 0..18 {
        assert!(parse(pump_amm::discriminators::CREATE_POOL, &body, &keys[..count]).is_none());
    }
    for len in 0..62 {
        let complete = matches!(len, 50 | 51 | 52 | 60 | 61);
        assert_eq!(
            parse(pump_amm::discriminators::CREATE_POOL, &body[..len], &keys).is_some(),
            complete,
            "len={len}"
        );
    }
    for offset in [50, 51, 60, 61] {
        for flag in 0..=255 {
            let mut data = body.clone();
            data[offset] = flag;
            assert_eq!(
                parse(pump_amm::discriminators::CREATE_POOL, &data, &keys).is_some(),
                flag <= 1,
                "offset={offset},flag={flag}"
            );
        }
    }
    let DexEvent::PumpSwapCreatePool(e) =
        parse(pump_amm::discriminators::CREATE_POOL, &body, &keys).unwrap()
    else {
        panic!("create")
    };
    assert_eq!(
        (
            e.index,
            e.base_amount_in,
            e.quote_amount_in,
            e.coin_creator,
            e.creator_fee_bps
        ),
        (42, 123, 456, keys[2], 250)
    );
}

#[test]
fn liquidity_cpi_and_logs_share_the_official_event_layout() {
    let mut body = Vec::new();
    for value in 1u64..=11 {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for value in 1u8..=5 {
        body.extend_from_slice(&[value; 32]);
    }
    for disc in [
        pump_amm_inner::discriminators::ADD_LIQUIDITY,
        pump_amm_inner::discriminators::REMOVE_LIQUIDITY,
    ] {
        let metadata = EventMetadata {
            slot: 1,
            ..Default::default()
        };
        let inner =
            pump_amm_inner::parse_pumpswap_inner_instruction(&disc, &body, metadata.clone())
                .unwrap();
        match &inner {
            DexEvent::PumpSwapLiquidityAdded(e) => assert_eq!(
                (
                    e.timestamp,
                    e.lp_token_amount_out,
                    e.lp_mint_supply,
                    e.pool,
                    e.user_pool_token_account
                ),
                (
                    1,
                    2,
                    11,
                    Pubkey::new_from_array([1; 32]),
                    Pubkey::new_from_array([5; 32])
                )
            ),
            DexEvent::PumpSwapLiquidityRemoved(e) => assert_eq!(
                (
                    e.timestamp,
                    e.lp_token_amount_in,
                    e.lp_mint_supply,
                    e.pool,
                    e.user_pool_token_account
                ),
                (
                    1,
                    2,
                    11,
                    Pubkey::new_from_array([1; 32]),
                    Pubkey::new_from_array([5; 32])
                )
            ),
            _ => panic!("liquidity"),
        }
        let mut wire = disc[8..].to_vec();
        wire.extend_from_slice(&body);
        let log = format!("Program data: {}", STANDARD.encode(wire));
        let decoded = logs::parse_log(&log, Default::default(), 1, 0, None, 0).unwrap();
        assert_eq!(
            serde_json::to_string(&inner).unwrap(),
            serde_json::to_string(&decoded).unwrap()
        );
        for len in 0..body.len() {
            assert!(
                pump_amm_inner::parse_pumpswap_inner_instruction(
                    &disc,
                    &body[..len],
                    metadata.clone()
                )
                .is_none(),
                "len={len}"
            );
            let mut wire = disc[8..].to_vec();
            wire.extend_from_slice(&body[..len]);
            assert!(
                logs::parse_log(
                    &format!("Program data: {}", STANDARD.encode(wire)),
                    Default::default(),
                    1,
                    0,
                    None,
                    0
                )
                .is_none(),
                "log_len={len}"
            );
        }
    }
}

#[test]
fn create_pool_cpi_uses_shared_layout_and_strict_bools() {
    let mut body = vec![0; 336];
    body[..8].copy_from_slice(&123i64.to_le_bytes());
    body[8..10].copy_from_slice(&42u16.to_le_bytes());
    body[165..197].copy_from_slice(&[7; 32]);
    body[326..334].copy_from_slice(&250u64.to_le_bytes());
    let disc = pump_amm_inner::discriminators::CREATE_POOL;
    let inner =
        pump_amm_inner::parse_pumpswap_inner_instruction(&disc, &body, EventMetadata::default())
            .unwrap();
    let DexEvent::PumpSwapCreatePool(e) = inner else {
        panic!("create")
    };
    assert_eq!(
        (e.timestamp, e.index, e.pool, e.creator_fee_bps),
        (123, 42, Pubkey::new_from_array([7; 32]), 250)
    );
    for offset in [325, 334, 335] {
        for flag in 0..=255 {
            let mut bytes = body.clone();
            bytes[offset] = flag;
            assert_eq!(
                logs::parse_create_pool_from_data(&bytes, EventMetadata::default()).is_some(),
                flag <= 1
            );
            assert_eq!(
                pump_amm_inner::parse_pumpswap_inner_instruction(
                    &disc,
                    &bytes,
                    EventMetadata::default()
                )
                .is_some(),
                flag <= 1
            );
        }
    }
}
