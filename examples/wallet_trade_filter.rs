use sol_parser_sdk::grpc::program_ids::get_program_ids_for_protocols;
use sol_parser_sdk::grpc::{Protocol, TransactionFilter};
use sol_parser_sdk::utils::normalize_buy_sell_from_token_delta;

fn main() {
    let protocols = [
        Protocol::RaydiumCpmm,
        Protocol::RaydiumClmm,
        Protocol::OrcaWhirlpool,
        Protocol::MeteoraDammV2,
        Protocol::MeteoraDlmm,
        Protocol::MeteoraDbc,
    ];
    let program_ids = get_program_ids_for_protocols(&protocols);
    let filter = TransactionFilter::from_program_ids(program_ids.clone())
        .require_account("Wallet111111111111111111111111111111111111");
    let side = normalize_buy_sell_from_token_delta(100);

    println!("scenario=wallet_trade_filter");
    println!("program_ids={program_ids:?}");
    println!("account_include={:?}", filter.account_include);
    println!("account_required={:?}", filter.account_required);
    println!("normalized_side={:?}", side.map(|s| s.as_str()));
}
