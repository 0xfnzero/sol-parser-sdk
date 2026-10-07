#[path = "support/pumpfun_rpc.rs"]
mod support;
use sol_parser_sdk::{parse_rpc_transaction, DexEvent};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::{fs, path::PathBuf};

#[test]
fn mainnet_create_account_regressions() {
    let directory =
        std::env::var_os("PUMPFUN_CREATE_CORPUS").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pumpfun_create")
        });
    let mut successful = 0;
    for file in fs::read_dir(directory).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        use base64::{engine::general_purpose::STANDARD, Engine};
        let mut binary = raw.clone();
        binary["transaction"] = serde_json::json!([STANDARD.encode(support::wire(&raw)), "base64"]);
        let tx: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(binary).unwrap();
        let events = parse_rpc_transaction(&tx, None).unwrap();
        if !raw["meta"]["err"].is_null() {
            assert!(events.is_empty(), "rolled-back events: {}", path.display());
            continue;
        }
        successful += 1;
        let creates: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                DexEvent::PumpFunCreate(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(!creates.is_empty(), "{}", path.display());
        let mut keys: Vec<_> = raw["transaction"]["message"]["accountKeys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap())
            .collect();
        for field in ["writable", "readonly"] {
            keys.extend(
                raw["meta"]["loadedAddresses"][field]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| k.as_str().unwrap()),
            );
        }
        for c in creates {
            let ix = raw["transaction"]["message"]["instructions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|ix| {
                    let data = bs58::decode(ix["data"].as_str().unwrap()).into_vec().unwrap();
                    keys[ix["programIdIndex"].as_u64().unwrap() as usize]
                        == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
                        && (data.starts_with(&[24, 30, 200, 40, 5, 28, 7, 119])
                            || data.starts_with(&[214, 144, 76, 236, 95, 139, 49, 180]))
                        && keys[ix["accounts"][0].as_u64().unwrap() as usize] == c.mint.to_string()
                })
                .expect("matching create instruction");
            let data = bs58::decode(ix["data"].as_str().unwrap()).into_vec().unwrap();
            let v2 = data.starts_with(&[214, 144, 76, 236, 95, 139, 49, 180]);
            assert_eq!(
                c.user.to_string(),
                keys[ix["accounts"][if v2 { 5 } else { 7 }].as_u64().unwrap() as usize],
                "{}",
                path.display()
            );
            assert_eq!(
                c.token_program.to_string(),
                keys[ix["accounts"][if v2 { 7 } else { 9 }].as_u64().unwrap() as usize],
                "{}",
                path.display()
            );
            assert!(
                [
                    "So11111111111111111111111111111111111111111",
                    "So11111111111111111111111111111111111111112"
                ]
                .contains(&c.quote_mint.to_string().as_str()),
                "{}",
                path.display()
            );
            assert_eq!(c.quote_vault, Pubkey::default());
        }
    }
    assert!(successful >= 2);
}

#[test]
fn historical_create_log_exact_layout() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let mut data = vec![27, 114, 169, 77, 222, 235, 99, 118];
    for value in ["name", "SYM", "uri"] {
        data.extend((value.len() as u32).to_le_bytes());
        data.extend(value.as_bytes());
    }
    for seed in [1, 2, 3] {
        data.extend([seed; 32]);
    }
    let parse = |bytes: &[u8]| {
        sol_parser_sdk::logs::pump::parse_log(
            &format!("Program data: {}", STANDARD.encode(bytes)),
            Signature::default(),
            1,
            0,
            None,
            0,
            false,
        )
    };
    let event = parse(&data).expect("historical event");
    match event {
        DexEvent::PumpFunCreate(c) => {
            assert_eq!(c.mint, Pubkey::new_from_array([1; 32]));
            assert_eq!(c.creator, Pubkey::default());
            assert_eq!(c.timestamp, 0);
        }
        _ => panic!("create"),
    }
    let mut disc = [0u8; 16];
    disc[..8].copy_from_slice(&data[..8]);
    disc[8..].copy_from_slice(&[155, 167, 108, 32, 122, 76, 173, 64]);
    assert!(matches!(
        sol_parser_sdk::instr::pump_inner::parse_pumpfun_inner_instruction(
            &disc,
            &data[8..],
            Default::default(),
            false
        ),
        Some(DexEvent::PumpFunCreate(_))
    ));
    assert!(matches!(
        sol_parser_sdk::logs::pump::parse_create_from_data(&data[8..], Default::default()),
        Some(DexEvent::PumpFunCreate(_))
    ));
    assert!(parse(&data[..data.len() - 1]).is_none());
    data.push(0);
    assert!(parse(&data).is_none());
}
