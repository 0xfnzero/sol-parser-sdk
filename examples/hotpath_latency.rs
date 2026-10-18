use sol_parser_sdk as sdk;
use solana_sdk::{
    message::{
        Message as WireMessage, VersionedMessage, compiled_instruction::CompiledInstruction,
    },
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
// Native local parser benchmark. No network/RPC, decoding and fixture signature
// verification are outside the timed loop. Use --release for runtime measurements.
use base64::{Engine, engine::general_purpose::STANDARD};
use sdk::core::events::DexEvent;
use sdk::grpc::{EventType, EventTypeFilter};
use sdk::shredstream::parse_transaction_dex_events_with_filter;
use sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    warmup: usize,
    samples: usize,
    batch: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self { warmup: 1000, samples: 1000, batch: 32 }
    }
}
struct Case {
    name: String,
    tx: VersionedTransaction,
    writable: Vec<Pubkey>,
    readonly: Vec<Pubkey>,
    expected: usize,
}
fn parse(case: &Case, filter: &EventTypeFilter, out: &mut Vec<DexEvent>) {
    out.clear();
    if case.writable.is_empty() && case.readonly.is_empty() {
        parse_transaction_dex_events_with_filter(
            black_box(&case.tx),
            case.tx.signatures.first().copied().unwrap_or_default(),
            42,
            7,
            999,
            Some(filter),
            out,
        );
    } else {
        parse_transaction_dex_events_with_loaded_addresses(
            black_box(&case.tx),
            &case.writable,
            &case.readonly,
            case.tx.signatures[0],
            42,
            7,
            999,
            Some(filter),
            out,
        )
        .unwrap();
    }
    black_box(&*out);
}
fn main() {
    let cfg: Config = std::env::args()
        .nth(1)
        .map(|p| serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
        .unwrap_or_default();
    assert!(
        (1..=100000).contains(&cfg.samples)
            && (1..=10000).contains(&cfg.batch)
            && cfg.warmup <= 1000000
    );
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../tests/fixtures/shred_routes/reported.json")).unwrap();
    let mut cases = Vec::new();
    for (i, f) in fixtures.iter().enumerate() {
        let tx: VersionedTransaction =
            wincode::deserialize_exact(&STANDARD.decode(f["wire"].as_str().unwrap()).unwrap())
                .unwrap();
        for (sig, key) in tx.signatures.iter().zip(tx.message.static_account_keys()) {
            assert!(sig.verify(key.as_ref(), &tx.message.serialize()));
        }
        let addresses = |name: &str| {
            f[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().parse().unwrap())
                .collect()
        };
        cases.push(Case {
            name: format!("signed_route_{i}"),
            tx,
            writable: addresses("loaded_writable"),
            readonly: addresses("loaded_readonly"),
            expected: f["expected"].as_array().unwrap().len(),
        });
    }
    for (name, program) in [
        ("unsupported_8x64", Pubkey::new_from_array([81; 32])),
        ("pump_unknown_8x64", sdk::instr::pump::PROGRAM_ID_PUBKEY),
    ] {
        let mut message = WireMessage::default();
        message.account_keys = vec![Pubkey::default(); 64];
        message.account_keys[63] = program;
        message.instructions = (0..8)
            .map(|_| CompiledInstruction {
                program_id_index: 63,
                accounts: (0..64).collect(),
                data: vec![0xAB; 24],
            })
            .collect();
        cases.push(Case {
            name: name.into(),
            tx: VersionedTransaction {
                signatures: vec![],
                message: VersionedMessage::Legacy(message),
            },
            writable: vec![],
            readonly: vec![],
            expected: 0,
        });
    }
    let filter =
        EventTypeFilter::include_only(vec![EventType::PumpFunCreate, EventType::PumpFunTrade]);
    let mut reports = Vec::new();
    for case in &cases {
        let mut out = Vec::new();
        parse(case, &filter, &mut out);
        assert_eq!(out.len(), case.expected, "{}", case.name);
        for _ in 0..cfg.warmup {
            parse(case, &filter, &mut out);
        }
        let mut times = Vec::with_capacity(cfg.samples);
        for _ in 0..cfg.samples {
            let start = Instant::now();
            for _ in 0..cfg.batch {
                parse(case, &filter, &mut out);
            }
            times.push(start.elapsed().as_nanos() as f64 / cfg.batch as f64);
        }
        times.sort_by(|a, b| a.total_cmp(b));
        let p = |q: f64| times[((times.len() - 1) as f64 * q) as usize];
        reports.push(json!({"case":case.name,"events":case.expected,"p50_ns":p(0.50),"p95_ns":p(0.95),"p99_ns":p(0.99)}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"sdk":env!("CARGO_PKG_NAME"),"profile":if cfg!(debug_assertions){"debug"}else{"release"},"warmup":cfg.warmup,"samples":cfg.samples,"batch":cfg.batch,"rpc_calls":0,"timings":reports})).unwrap());
}
