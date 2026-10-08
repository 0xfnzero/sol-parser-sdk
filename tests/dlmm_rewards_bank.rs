use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn current_bank_dlmm_rewards_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/dlmm_rewards_20261008.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} rollback", c["name"]);
            continue;
        }
        // Keep fee quantities exact, including values beyond JS safe integers.
        let actual: Vec<_> = parsed
            .events
            .iter()
            .filter_map(|e| match e {
                DexEvent::MeteoraDlmmClaimReward(e) => Some((
                    "MeteoraDlmmClaimReward",
                    fields!(e, pool, position, owner, reward_index, total_reward, active_bin_id),
                )),
                _ => None,
            })
            .collect();
        let wants = c["expected"].as_array().unwrap();
        assert_eq!(actual.len(), wants.len(), "{}", c["name"]);
        for ((kind, body), want) in actual.iter().zip(wants) {
            assert_eq!(*kind, want["type"].as_str().unwrap());
            for (k, v) in want.as_object().unwrap() {
                if k != "type" {
                    assert_eq!(body.get(k).unwrap(), v.as_str().unwrap(), "{} {k}", c["name"]);
                }
            }
        }
    }
}

#[test]
fn reward_claim_logs_filters_truncation_and_u64_precision() {
    // Optimized matcher debug builds have large stack frames.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(check_reward_claim_logs)
        .unwrap()
        .join()
        .unwrap();
}

fn check_reward_claim_logs() {
    use base64::Engine;
    use sol_parser_sdk::core::events::EventMetadata;
    use sol_parser_sdk::grpc::types::{EventType, EventTypeFilter};
    use sol_parser_sdk::instr::all_inner::meteora_dlmm;
    use sol_parser_sdk::logs::optimized_matcher::{
        parse_log_optimized, parse_log_optimized_with_program_id,
    };
    use solana_sdk::{pubkey::Pubkey, signature::Signature};
    let mut body = vec![27, 143, 244, 33, 80, 43, 110, 146];
    body.extend((1..=96).map(|n| n as u8));
    body.extend(1u64.to_le_bytes());
    body.extend(u64::MAX.to_le_bytes());
    body.extend((-123i32).to_le_bytes());
    let program: Pubkey = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".parse().unwrap();
    let include = EventTypeFilter::include_only(vec![EventType::MeteoraDlmmClaimReward]);
    let log =
        |b: &[u8]| format!("Program data: {}", base64::engine::general_purpose::STANDARD.encode(b));
    for n in 0..body.len() {
        assert!(
            parse_log_optimized_with_program_id(
                &log(&body[..n]),
                Signature::default(),
                0,
                0,
                None,
                0,
                Some(&include),
                false,
                None,
                Some(&program)
            )
            .is_none(),
            "truncated {n}"
        );
    }
    for event in [
        parse_log_optimized_with_program_id(
            &log(&body),
            Signature::default(),
            0,
            0,
            None,
            0,
            Some(&include),
            false,
            None,
            Some(&program),
        ),
        parse_log_optimized(
            &log(&body),
            Signature::default(),
            0,
            0,
            None,
            0,
            Some(&include),
            false,
            None,
        ),
        meteora_dlmm::parse(
            &[228, 69, 165, 46, 81, 203, 154, 29, 27, 143, 244, 33, 80, 43, 110, 146],
            &body[8..],
            EventMetadata::default(),
        ),
    ] {
        match event.unwrap() {
            DexEvent::MeteoraDlmmClaimReward(e) => {
                assert_eq!(e.reward_index, 1);
                assert_eq!(e.active_bin_id, -123);
                assert_eq!(e.total_reward, u64::MAX);
            }
            _ => panic!("wrong event"),
        }
    }
    let exclude = EventTypeFilter::exclude_types(vec![EventType::MeteoraDlmmClaimReward]);
    assert!(parse_log_optimized_with_program_id(
        &log(&body),
        Signature::default(),
        0,
        0,
        None,
        0,
        Some(&exclude),
        false,
        None,
        Some(&program)
    )
    .is_none());
}
