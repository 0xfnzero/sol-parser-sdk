use sol_parser_sdk::parse_rpc_transaction_with_cost;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
#[test]
fn deployed_two_hop_venue_combinations() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("fixtures/pump_routes_20261009.json")).unwrap();
    for c in fixture["cases"].as_array().unwrap() {
        let tx:EncodedConfirmedTransactionWithStatusMeta=serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed=parse_rpc_transaction_with_cost(&tx,None).unwrap();
        let actual:Vec<_>=parsed.events.iter().filter_map(|e| {
            let json=serde_json::to_value(e).unwrap();
            ["PumpFunTrade","PumpSwapBuy","PumpSwapSell"].into_iter().find_map(|kind| json.get(kind).map(|body|(kind.to_owned(),body.clone())))
        }).collect();
        let wants=c["expected"].as_array().unwrap();assert_eq!(actual.len(),wants.len(),"{}",c["name"]);
        for ((kind,body),want)in actual.iter().zip(wants) {
            assert_eq!(kind,want["type"].as_str().unwrap());
            for(k,v)in want.as_object().unwrap(){if k=="type"{continue;}let a=&body[k];let text=if let Some(bytes)=a.as_array(){let bytes:Vec<u8>=bytes.iter().map(|b|b.as_u64().unwrap()as u8).collect();solana_sdk::pubkey::Pubkey::new_from_array(bytes.try_into().unwrap()).to_string()}else{a.as_str().map(str::to_owned).unwrap_or_else(||a.to_string())};assert_eq!(text,v.as_str().unwrap(),"{} {k}",c["name"]);}
        }
    }
}
