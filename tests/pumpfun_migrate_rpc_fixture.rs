//! PumpFun migrations from saved mainnet transactions. `migrate_v2` emits
//! `CompletePumpAmmMigrationEvent` both as a `Program data:` log line and as an
//! event CPI. The log line comes after the pump AMM CPIs, past the 10 000 bytes
//! of logs a validator keeps by default, so a stream from such a validator
//! carries the event only as the CPI.

use base64::Engine as _;
use sol_parser_sdk::core::events::PumpFunMigrateEvent;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::{parse_rpc_transaction, DexEvent};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

/// `migrate_v2` of a SOL-quoted curve into its pump AMM pool, with 12 525
/// bytes of logs.
const MIGRATE_V2: &str = include_str!("fixtures/pumpfun_migrate_v2_rpc_transaction.json");
/// A repeated `migrate_v2` that succeeds without migrating: "Bonding curve
/// already migrated".
const MIGRATE_V2_ALREADY_MIGRATED: &str =
    include_str!("fixtures/pumpfun_migrate_v2_already_migrated_rpc_transaction.json");

/// The bytes of log messages a validator keeps per transaction by default.
const DEFAULT_LOG_BYTES_LIMIT: usize = 10_000;
const MIGRATION_EVENT_DISCRIMINATOR: [u8; 8] = [189, 233, 93, 185, 92, 148, 234, 148];

fn pk(key: &str) -> Pubkey {
    key.parse().expect("valid pubkey")
}

fn fixture(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("valid RPC transaction fixture")
}

/// Cuts the transaction's logs the way a validator's log collector does: a
/// message that would reach the limit is dropped, and the first one dropped
/// leaves "Log truncated".
fn truncate_logs(transaction: &mut serde_json::Value, limit: usize) {
    let logs = transaction["meta"]["logMessages"].as_array_mut().expect("log messages");
    let mut written = 0;
    let mut truncated = false;
    let mut kept = Vec::new();
    for log in logs.iter() {
        let message = log.as_str().expect("log message");
        if written + message.len() < limit {
            written += message.len();
            kept.push(log.clone());
        } else if !truncated {
            truncated = true;
            kept.push("Log truncated".into());
        }
    }
    *logs = kept;
}

fn logs(transaction: &serde_json::Value) -> Vec<&str> {
    transaction["meta"]["logMessages"]
        .as_array()
        .expect("log messages")
        .iter()
        .map(|log| log.as_str().expect("log message"))
        .collect()
}

fn is_migration_event_log(log: &str) -> bool {
    log.strip_prefix("Program data: ")
        .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok())
        .is_some_and(|data| data.starts_with(&MIGRATION_EVENT_DISCRIMINATOR))
}

fn migrations(
    transaction: serde_json::Value,
    filter: Option<&EventTypeFilter>,
) -> Vec<PumpFunMigrateEvent> {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_value(transaction).expect("RPC transaction");
    parse_rpc_transaction(&transaction, filter)
        .expect("parse RPC fixture")
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::PumpFunMigrate(migrate) => Some(migrate),
            _ => None,
        })
        .collect()
}

/// The migration in `MIGRATE_V2`, as its event reports it.
fn assert_migrate_v2(migrate: &PumpFunMigrateEvent) {
    assert_eq!(
        migrate.metadata.signature.to_string(),
        "3gsRn6ZLtwFjf7umdHYnzUz7NkWx45nrhm3u4MpdfHsC6VnRtXB5J9XMvnN6h7zWLc5raXhb4e4YXwn3wyTszb9n"
    );
    assert_eq!(migrate.metadata.slot, 454_176_142);
    assert_eq!(migrate.user, pk("DfzpdHzWnX7WAdiGDNLtWNJsyKswjA1kEBcVSMtRJ8iE"));
    assert_eq!(migrate.mint, pk("BB5DMEmCSC2nH2nJNdcsxKPQcU6UWHDQF2mDKiJkh5xp"));
    assert_eq!(migrate.mint_amount, 206_900_000_000_000);
    assert_eq!(migrate.sol_amount, 84_990_359_056);
    assert_eq!(migrate.pool_migration_fee, 15_000_001);
    assert_eq!(migrate.bonding_curve, pk("GNtLotVdQqyRB72Qwk7UfpsyzywU9pw5zv4Saq5JRjQ3"));
    assert_eq!(migrate.timestamp, 1_791_363_549);
    assert_eq!(migrate.pool, pk("heUt5ff16SBzPC8R6rfce9RFTyP6VSvwJkHGQZbfSa5"));
}

#[test]
fn migrate_v2_with_full_logs_is_one_migration() {
    let transaction = fixture(MIGRATE_V2);
    assert!(logs(&transaction).iter().any(|log| is_migration_event_log(log)));

    let migrations = migrations(transaction, None);

    assert_eq!(migrations.len(), 1, "the log line and the event CPI are one migration");
    assert_migrate_v2(&migrations[0]);
}

#[test]
fn migrate_v2_with_logs_cut_at_the_default_limit_is_parsed_from_its_event_cpi() {
    let mut transaction = fixture(MIGRATE_V2);
    truncate_logs(&mut transaction, DEFAULT_LOG_BYTES_LIMIT);
    let kept = logs(&transaction);
    assert!(kept.contains(&"Log truncated"));
    assert!(!kept.iter().any(|log| is_migration_event_log(log)), "the event's log line is cut");

    let only_migrations = EventTypeFilter::include_only(vec![EventType::PumpFunMigrate]);
    for filter in [None, Some(&only_migrations)] {
        let migrations = migrations(transaction.clone(), filter);

        assert_eq!(migrations.len(), 1);
        assert_migrate_v2(&migrations[0]);
    }
}

#[test]
fn migrate_v2_of_an_already_migrated_curve_is_no_migration() {
    let transaction = fixture(MIGRATE_V2_ALREADY_MIGRATED);
    assert!(logs(&transaction).contains(&"Program log: Bonding curve already migrated"));

    assert!(migrations(transaction, None).is_empty());
}
