use sol_parser_sdk::parse_rpc_transaction_with_cost;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
#[test]
fn deployed_holder_rewards() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pump_holder_20261009.json")).unwrap();
    for c in fixture["cases"].as_array().unwrap() {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        // Verify versioned instruction names against wire evidence, independently
        // of the shorter names in emitted program logs.
        let wire = tx.transaction.transaction.decode().unwrap();
        for want in c["expected"].as_array().unwrap() {
            let discriminator: Option<[u8; 8]> = match want["ix_name"].as_str() {
                Some("buy_exact_quote_in_v3") => Some([225, 247, 80, 30, 213, 179, 132, 136]),
                Some("sell_v3") => Some([28, 146, 222, 119, 38, 196, 105, 213]),
                _ => None,
            };
            if let Some(discriminator) = discriminator {
                assert!(wire.message.instructions().iter().any(|ix| {
                    wire.message.static_account_keys()[ix.program_id_index as usize]
                        == sol_parser_sdk::instr::program_ids::PUMPFUN_PROGRAM_ID
                        && ix.data.starts_with(&discriminator)
                }));
            }
        }
        let actual: Vec<_> = parsed
            .events
            .iter()
            .filter_map(|e| {
                let json = serde_json::to_value(e).unwrap();
                ["PumpFunBuy", "PumpFunSell", "PumpSwapBuy", "PumpSwapSell"]
                    .into_iter()
                    .find_map(|kind| json.get(kind).map(|body| (kind.to_owned(), body.clone())))
            })
            .collect();
        let wants = c["expected"].as_array().unwrap();
        assert_eq!(actual.len(), wants.len(), "{}", c["name"]);
        for ((kind, body), want) in actual.iter().zip(wants) {
            assert_eq!(kind, want["type"].as_str().unwrap());
            for (k, v) in want.as_object().unwrap() {
                if k == "type" {
                    continue;
                }
                let a = &body[k];
                let text = if let Some(bytes) = a.as_array() {
                    let bytes: Vec<u8> = bytes.iter().map(|b| b.as_u64().unwrap() as u8).collect();
                    solana_sdk::pubkey::Pubkey::new_from_array(bytes.try_into().unwrap())
                        .to_string()
                } else {
                    a.as_str().map(str::to_owned).unwrap_or_else(|| a.to_string())
                };
                assert_eq!(text, v.as_str().unwrap(), "{} {k}", c["name"]);
            }
        }
    }
}
