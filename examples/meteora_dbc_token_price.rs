use sol_parser_sdk::grpc::program_ids::METEORA_DBC_PROGRAM_ID;
use sol_parser_sdk::utils::vault_price_from_balances;

fn main() {
    let price = vault_price_from_balances(1_000_000_000, 2_000_000, 9, 6);

    println!("scenario=meteora_dbc_token_price");
    println!("program_id={METEORA_DBC_PROGRAM_ID}");
    println!("price={price:?}");
}
