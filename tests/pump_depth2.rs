use sol_parser_sdk::{parse_rpc_transaction, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
#[test]
fn signed_depth2_bank_replay() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pump_depth2_20261014.json")).unwrap();
    for c in f["cases"].as_array().unwrap() {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let events = parse_rpc_transaction(&tx, None).unwrap();
        let creates: Vec<_> = events
            .iter()
            .filter_map(|e| if let DexEvent::PumpFunCreate(e) = e { Some(e) } else { None })
            .collect();
        assert_eq!(creates.len(), 1, "{}", c["name"]);
        let e = creates[0];
        let expected = &c["expected"];
        assert_eq!(e.mint.to_string(), expected["mint"].as_str().unwrap());
        assert_eq!(e.quote_mint.to_string(), expected["quote_mint"].as_str().unwrap());
        assert_eq!(e.depth, 2);
        assert_eq!(
            e.virtual_quote_reserves.to_string(),
            expected["virtual_quote_reserves"].as_str().unwrap()
        );
    }
}
