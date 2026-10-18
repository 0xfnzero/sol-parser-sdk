//! cargo run --offline --example pump_offline_bank
mod offline_bank;
fn main() {
    let config = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    offline_bank::run(config.as_deref());
}
