//! The example and integration test execute the same native Rust bank harness.
#[path = "../examples/offline_bank/mod.rs"]
mod offline_bank;
#[test]
fn freshly_signed_routes_execute_and_parse_repeatably_without_rpc() {
    assert_eq!(offline_bank::run(None), 20);
}
