//! Local signed execution and parsing. No client or network API is used.
use base64::{engine::general_purpose::STANDARD, Engine};
use litesvm::{types::TransactionMetadata, LiteSVM};
use sdk::core::events::DexEvent;
use serde_json::{json, Value};
use sol_parser_sdk as sdk;
use solana_account::Account;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_sdk::{
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

type State = BTreeMap<Pubkey, Option<Account>>;
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/offline_bank");
fn key(s: &str) -> Pubkey {
    s.parse().unwrap()
}
fn amm() -> Pubkey {
    key("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
}
fn pump() -> Pubkey {
    key("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
}
fn token(p: Pubkey) -> bool {
    [
        key("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        key("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
    ]
    .contains(&p)
}
fn bytes(v: &Value) -> Vec<u8> {
    STANDARD.decode(v.as_str().unwrap()).unwrap()
}
fn n(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[..8].try_into().unwrap())
}
fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn snapshot(svm: &LiteSVM, tracked: &BTreeSet<Pubkey>) -> State {
    tracked.iter().map(|k| (*k, svm.get_account(k))).collect()
}
fn ata(user: Pubkey, owner: Pubkey, mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[user.as_ref(), owner.as_ref(), mint.as_ref()],
        &key("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
    )
    .0
}

fn initialize(
    bundle: &Value,
    case: &Value,
    wallet: &Keypair,
    alt: bool,
) -> (LiteSVM, Instruction, Vec<AddressLookupTableAccount>, BTreeSet<Pubkey>) {
    let msg = &case["raw"]["transaction"]["message"];
    let keys: Vec<_> =
        msg["accountKeys"].as_array().unwrap().iter().map(|v| key(v.as_str().unwrap())).collect();
    let original = msg["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| keys[i["programIdIndex"].as_u64().unwrap() as usize] == amm())
        .unwrap();
    let old = keys[original["accounts"][0].as_u64().unwrap() as usize];
    let mut mapping = BTreeMap::from([(old, wallet.pubkey())]);
    for program in [amm(), pump()] {
        mapping.insert(
            Pubkey::find_program_address(&[b"user_volume_accumulator", old.as_ref()], &program).0,
            Pubkey::find_program_address(
                &[b"user_volume_accumulator", wallet.pubkey().as_ref()],
                &program,
            )
            .0,
        );
    }
    let accounts = case["accounts"].as_object().unwrap();
    for (address, a) in accounts {
        let data = bytes(&a["data"][0]);
        let owner = key(a["owner"].as_str().unwrap());
        if token(owner) && data.len() >= 165 && data[32..64] == old.to_bytes() {
            let mint = Pubkey::new_from_array(data[..32].try_into().unwrap());
            if ata(old, owner, mint) == key(address) {
                mapping.insert(key(address), ata(wallet.pubkey(), owner, mint));
            }
        }
    }
    let mapped = |p: Pubkey| *mapping.get(&p).unwrap_or(&p);
    let mut svm =
        LiteSVM::new().with_sigverify(true).with_blockhash_check(true).with_log_bytes_limit(None);
    let clock = bundle["clock"].as_array().unwrap();
    svm.set_sysvar(&Clock {
        slot: clock[0].as_u64().unwrap(),
        epoch_start_timestamp: clock[1].as_i64().unwrap(),
        epoch: clock[2].as_u64().unwrap(),
        leader_schedule_epoch: clock[3].as_u64().unwrap(),
        unix_timestamp: clock[4].as_i64().unwrap(),
    });
    for (p, code) in bundle["programs"].as_object().unwrap() {
        svm.add_program(key(p), &bytes(code)).unwrap();
    }
    let mut tracked = BTreeSet::new();
    for (address, a) in accounts {
        let mut data = bytes(&a["data"][0]);
        // Replace only exact captured public-key occurrences in this test state.
        let original_data = data.clone();
        for (from, to) in &mapping {
            for offset in 0..original_data.len().saturating_sub(31) {
                if original_data[offset..offset + 32] == from.to_bytes() {
                    data[offset..offset + 32].copy_from_slice(to.as_ref());
                }
            }
        }
        let address = mapped(key(address));
        svm.set_account(
            address,
            Account {
                lamports: a["lamports"].as_u64().unwrap(),
                data,
                owner: key(a["owner"].as_str().unwrap()),
                executable: a["executable"].as_bool().unwrap(),
                rent_epoch: a["rentEpoch"].as_u64().unwrap(),
            },
        )
        .unwrap();
        tracked.insert(address);
    }
    svm.airdrop(&wallet.pubkey(), 500_000_000_000).unwrap();
    let h = &msg["header"];
    let ns = h["numRequiredSignatures"].as_u64().unwrap() as usize;
    let rs = h["numReadonlySignedAccounts"].as_u64().unwrap() as usize;
    let ru = h["numReadonlyUnsignedAccounts"].as_u64().unwrap() as usize;
    let metas: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| AccountMeta {
            pubkey: mapped(*k),
            is_signer: i < ns,
            is_writable: if i < ns { i < ns - rs } else { i < keys.len() - ru },
        })
        .collect();
    let ix = Instruction {
        program_id: amm(),
        data: bs58::decode(original["data"].as_str().unwrap()).into_vec().unwrap(),
        accounts: original["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| metas[i.as_u64().unwrap() as usize].clone())
            .collect(),
    };
    tracked.extend(ix.accounts.iter().map(|a| a.pubkey));
    tracked.insert(wallet.pubkey());
    let mut tables = Vec::new();
    if alt {
        let mut addresses = Vec::new();
        for a in ix.accounts.iter().filter(|a| !a.is_signer) {
            if !addresses.contains(&a.pubkey) {
                addresses.push(a.pubkey);
            }
        }
        let table_key = Pubkey::new_from_array([118; 32]);
        let mut data = vec![0; 56];
        data[..4].copy_from_slice(&1u32.to_le_bytes());
        data[4..12].copy_from_slice(&u64::MAX.to_le_bytes());
        for address in &addresses {
            data.extend_from_slice(address.as_ref());
        }
        svm.set_account(
            table_key,
            Account {
                lamports: svm.minimum_balance_for_rent_exemption(data.len()),
                data,
                owner: key("AddressLookupTab1e1111111111111111111111111"),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
        tables.push(AddressLookupTableAccount { key: table_key, addresses });
        tracked.insert(table_key);
    }
    (svm, ix, tables, tracked)
}

fn sign(
    svm: &LiteSVM,
    ix: &Instruction,
    tables: &[AddressLookupTableAccount],
    wallet: &Keypair,
) -> VersionedTransaction {
    let budget = Instruction {
        program_id: key("ComputeBudget111111111111111111111111111111"),
        accounts: vec![],
        data: [vec![2], 1_400_000u32.to_le_bytes().to_vec()].concat(),
    };
    let msg = v0::Message::try_compile(
        &wallet.pubkey(),
        &[budget, ix.clone()],
        tables,
        svm.latest_blockhash(),
    )
    .unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[wallet]).unwrap();
    tx.sanitize().unwrap();
    tx.verify_and_hash_message().unwrap();
    tx
}

fn projection(
    tx: &VersionedTransaction,
    meta: &TransactionMetadata,
    err: Value,
    tables: &[AddressLookupTableAccount],
    before: &State,
    after: &State,
    clock: &Value,
) -> Value {
    let VersionedMessage::V0(msg) = &tx.message else { unreachable!() };
    let loaded = |writable: bool| -> Vec<Pubkey> {
        msg.address_table_lookups
            .iter()
            .flat_map(|l| {
                let table = tables.iter().find(|t| t.key == l.account_key).unwrap();
                (if writable { &l.writable_indexes } else { &l.readonly_indexes })
                    .iter()
                    .map(|i| table.addresses[*i as usize])
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let writable = loaded(true);
    let readonly = loaded(false);
    let keys: Vec<_> = msg.account_keys.iter().chain(&writable).chain(&readonly).copied().collect();
    let lamports = |s: &State| -> Vec<_> {
        keys.iter().map(|k| s.get(k).and_then(|a| a.as_ref()).map_or(0, |a| a.lamports)).collect()
    };
    let balances = |s: &State| -> Vec<_> {
        keys.iter().enumerate().filter_map(|(i,k)| {
        let a = s.get(k)?.as_ref()?;
        if !token(a.owner) || a.data.len() < 165 || ![1,2].contains(&a.data[108]) { return None; }
        let mint = Pubkey::new_from_array(a.data[..32].try_into().unwrap());
        let decimals = s.get(&mint).and_then(|a| a.as_ref()).map_or(0, |a| a.data[44]);
        let amount = n(&a.data[64..]).to_string();
        Some(json!({"accountIndex":i,"mint":mint.to_string(),"owner":Pubkey::new_from_array(a.data[32..64].try_into().unwrap()).to_string(),"programId":a.owner.to_string(),"uiTokenAmount":{"amount":amount,"decimals":decimals,"uiAmount":null,"uiAmountString":amount}}))
    }).collect()
    };
    let groups: Vec<_> = meta.inner_instructions.iter().enumerate().filter(|(_, items)| !items.is_empty()).map(|(i,items)| {
        let instructions: Vec<_> = items.iter().map(|x| json!({"programIdIndex":x.instruction.program_id_index,"accounts":x.instruction.accounts,"data":bs58::encode(&x.instruction.data).into_string(),"stackHeight":x.stack_height})).collect();
        json!({"index":i,"instructions":instructions}) }).collect();
    json!({"slot":clock[0],"blockTime":clock[4],"version":0,"transaction":[STANDARD.encode(wincode::serialize(tx).unwrap()),"base64"],
        "meta":{"err":err,"status":if err.is_null(){json!({"Ok":null})}else{json!({"Err":err})},"fee":meta.fee,
            "preBalances":lamports(before),"postBalances":lamports(after),"preTokenBalances":balances(before),"postTokenBalances":balances(after),
            "loadedAddresses":{"writable":writable.iter().map(ToString::to_string).collect::<Vec<_>>(),"readonly":readonly.iter().map(ToString::to_string).collect::<Vec<_>>()},
            "innerInstructions":groups,"logMessages":meta.logs,"rewards":[]}})
}

fn balance(state: &State, k: Pubkey) -> u64 {
    state
        .get(&k)
        .and_then(|a| a.as_ref())
        .filter(|a| token(a.owner) && a.data.len() >= 165)
        .map_or(0, |a| n(&a.data[64..]))
}
fn execute(bundle: &Value, case: &Value, alt: bool, reject: bool, expected: &Value) -> Value {
    let wallet = Keypair::new_from_array([117; 32]);
    let (mut svm, mut ix, tables, mut tracked) = initialize(bundle, case, &wallet, alt);
    if reject {
        ix.data[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
    }
    let tx = sign(&svm, &ix, &tables, &wallet);
    tracked.extend(tx.message.static_account_keys());
    let before = snapshot(&svm, &tracked);
    let result = svm.simulate_transaction(tx.clone());
    assert_eq!(result.is_ok(), !reject, "{} {result:?}", case["name"]);
    assert!(before == snapshot(&svm, &tracked), "simulation changed state");
    let meta = match &result {
        Ok(r) => &r.meta,
        Err(e) => &e.meta,
    }
    .clone();
    let err = match &result {
        Ok(_) => Value::Null,
        Err(e) => serde_json::to_value(&e.err).unwrap(),
    };
    if reject {
        assert_eq!(err, json!({"InstructionError":[1,{"Custom":6004}]}));
        assert!(
            meta.logs.iter().any(|s| s == &format!("Program {} success", pump()))
                || case["name"] == "pool_pool_buy"
        );
    }
    let commit = svm.send_transaction(tx.clone());
    assert_eq!(commit.is_ok(), !reject);
    let after = snapshot(&svm, &tracked);
    if reject {
        assert!(
            before.iter().all(|(k, a)| *k == wallet.pubkey() || after.get(k) == Some(a)),
            "failed parent leaked state"
        );
        assert_eq!(
            before[&wallet.pubkey()].as_ref().unwrap().lamports
                - after[&wallet.pubkey()].as_ref().unwrap().lamports,
            meta.fee
        );
    } else {
        assert!(
            before.iter().any(|(k, a)| *k != wallet.pubkey() && after.get(k) != Some(a)),
            "successful route changed no state"
        );
        for (k, a) in &result.as_ref().unwrap().post_accounts {
            if tracked.contains(k) {
                assert!(after[k].as_ref() == Some(&Account::from(a.clone())));
            }
        }
        let change = after[&wallet.pubkey()].as_ref().unwrap().lamports as i128
            - before[&wallet.pubkey()].as_ref().unwrap().lamports as i128
            + meta.fee as i128;
        assert!(
            balance(&before, ix.accounts[1].pubkey)
                .saturating_sub(balance(&after, ix.accounts[1].pubkey))
                >= n(&ix.data[8..])
                || -change >= n(&ix.data[8..]) as i128,
            "input was not debited"
        );
        assert!(
            balance(&after, ix.accounts[2].pubkey) > balance(&before, ix.accounts[2].pubkey)
                || change > 0,
            "output was not credited"
        );
    }
    let mut invalid = tx.clone();
    let mut sig = invalid.signatures[0].as_ref().to_vec();
    sig[0] ^= 1;
    invalid.signatures[0] = Signature::from(<[u8; 64]>::try_from(sig).unwrap());
    let bad = svm.simulate_transaction(invalid).unwrap_err();
    assert_eq!(serde_json::to_value(bad.err).unwrap(), json!("SignatureFailure"));
    assert!(bad.meta.logs.is_empty());
    if alt {
        let VersionedMessage::V0(msg) = &tx.message else { unreachable!() };
        assert!(
            !msg.address_table_lookups[0].writable_indexes.is_empty()
                && !msg.address_table_lookups[0].readonly_indexes.is_empty()
        );
        for writable in [true, false] {
            let mut msg = msg.clone();
            let l = &mut msg.address_table_lookups[0];
            if writable {
                l.writable_indexes[0] = 255;
            } else {
                l.readonly_indexes[0] = 255;
            }
            let invalid =
                VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&wallet]).unwrap();
            invalid.verify_and_hash_message().unwrap();
            let bad = svm.simulate_transaction(invalid).unwrap_err();
            assert_eq!(
                serde_json::to_value(bad.err).unwrap(),
                json!("InvalidAddressLookupTableIndex")
            );
            assert!(bad.meta.logs.is_empty());
        }
    }
    assert!(after == snapshot(&svm, &tracked), "pre-execution rejection changed state");
    json!({"name":format!("{}/{}/{}",case["name"].as_str().unwrap(),if alt {"alt"}else{"static"},if reject {"slippage"}else{"success"}),
        "rpc":projection(&tx,&meta,err,&tables,&before,&after,&bundle["clock"]),"succeeded":!reject,
        "input":ix.accounts[1].pubkey.to_string(),"output":ix.accounts[2].pubkey.to_string(),"input_amount":n(&ix.data[8..]),"min_output":n(&ix.data[16..]),"expected":expected})
}

/// Run the configured matrix twice (or more), including SDK parsing each time.
pub fn run(config_path: Option<&Path>) -> usize {
    let default = Path::new(ROOT).join("config.json");
    let config = read_json(config_path.unwrap_or(&default));
    let payload =
        std::fs::read(Path::new(ROOT).join(config["snapshot"].as_str().unwrap())).unwrap();
    let hash = solana_sha256_hasher::hash(&payload)
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(hash, config["sha256"].as_str().unwrap(), "snapshot checksum mismatch");
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(payload.as_slice()).read_to_end(&mut data).unwrap();
    let bundle: Value = serde_json::from_slice(&data).unwrap();
    assert_eq!(bundle["schema"], 1);
    let expected = read_json(Path::new(ROOT).join("expected.json"));
    let repetitions = config["repetitions"].as_u64().unwrap();
    assert!(repetitions >= 2);
    let mut baseline = None;
    let mut count = 0;
    for repetition in 0..repetitions {
        let mut cases = Vec::new();
        for case in bundle["cases"].as_array().unwrap() {
            for alt in [false, true] {
                for reject in [false, true] {
                    cases.push(execute(
                        &bundle,
                        case,
                        alt,
                        reject,
                        &expected[case["name"].as_str().unwrap()],
                    ));
                }
            }
        }
        count = cases.len();
        assert_eq!(count, 20);
        check(&cases);
        if let Some(previous) = &baseline {
            assert_eq!(previous, &cases, "execution changed across repetitions");
        }
        baseline = Some(cases);
        println!(
            "Offline bank pass {}: {count} signed routes + SDK parsing passed",
            repetition + 1
        );
    }
    count
}

fn field_text(v: &Value) -> String {
    if let Some(bytes) = v.as_array() {
        let bytes: Vec<u8> = bytes.iter().map(|v| v.as_u64().unwrap() as u8).collect();
        Pubkey::new_from_array(bytes.try_into().unwrap()).to_string()
    } else {
        v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())
    }
}

fn check(cases: &[Value]) {
    for case in cases {
        let rpc = &case["rpc"];
        let tx: VersionedTransaction = wincode::deserialize_exact(
            &STANDARD.decode(rpc["transaction"][0].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        tx.sanitize().unwrap();
        tx.verify_and_hash_message().unwrap();
        let loaded = |side: &str| -> Vec<Pubkey> {
            rpc["meta"]["loadedAddresses"][side]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| k.as_str().unwrap().parse().unwrap())
                .collect()
        };
        let mut events = Vec::new();
        sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &tx,
            &loaded("writable"),
            &loaded("readonly"),
            tx.signatures[0],
            rpc["slot"].as_u64().unwrap(),
            0,
            0,
            None,
            &mut events,
        )
        .unwrap();
        assert_eq!(events.len(), 1, "{}", case["name"]);
        let DexEvent::PumpRouteIntent(intent) = &events[0] else {
            panic!("raw wire must yield intent")
        };
        assert_eq!(intent.hops.len(), 2);
        assert_eq!(intent.instruction_index, 1);
        assert_eq!(intent.input_account.to_string(), case["input"].as_str().unwrap());
        assert_eq!(intent.output_account.to_string(), case["output"].as_str().unwrap());
        assert_eq!(intent.amount_in, case["input_amount"].as_u64());
        assert_eq!(intent.min_amount_out, case["min_output"].as_u64());
        // Execution success/failure cannot change evidence available in raw shreds.
        let expected = case["expected"].as_array().unwrap();
        let mut fills = Vec::new();
        for line in rpc["meta"]["logMessages"].as_array().unwrap() {
            let line = line.as_str().unwrap();
            let event = sdk::logs::pump::parse_log(line, tx.signatures[0], 0, 0, None, 0, false)
                .or_else(|| sdk::logs::pump_amm::parse_log(line, tx.signatures[0], 0, 0, None, 0));
            if let Some(event) = event {
                let json = serde_json::to_value(event).unwrap();
                for kind in ["PumpFunTrade", "PumpSwapBuy", "PumpSwapSell"] {
                    if let Some(body) = json.get(kind) {
                        fills.push((kind, body.clone()));
                    }
                }
            }
        }
        assert_eq!(fills.len(), expected.len(), "{}", case["name"]);
        for ((kind, body), expected) in fills.iter().zip(expected) {
            assert_eq!(*kind, expected["type"].as_str().unwrap());
            for (field, value) in expected.as_object().unwrap() {
                if field == "type" {
                    continue;
                }
                let want = if field == "user" {
                    intent.user.to_string()
                } else {
                    value.as_str().unwrap().to_owned()
                };
                assert_eq!(field_text(&body[field]), want, "{} {field}", case["name"]);
            }
        }
        check_settlement(rpc, case["succeeded"].as_bool().unwrap());
    }
}

fn check_settlement(rpc: &Value, succeeded: bool) {
    let encoded = serde_json::from_value(rpc.clone()).unwrap();
    let parsed = sdk::parse_rpc_transaction_with_cost(&encoded, None).unwrap();
    let fills = parsed
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                DexEvent::PumpFunTrade(_) | DexEvent::PumpSwapBuy(_) | DexEvent::PumpSwapSell(_)
            )
        })
        .count();
    assert_eq!(fills, if succeeded { 2 } else { 0 }, "failed parent must not settle inner fills");
}
