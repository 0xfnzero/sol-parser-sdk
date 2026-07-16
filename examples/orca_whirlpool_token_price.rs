use sol_parser_sdk::grpc::program_ids::ORCA_WHIRLPOOL_PROGRAM_ID;
use sol_parser_sdk::utils::sqrt_price_x64_to_price;

fn main() {
    let sqrt_price_x64 = 1_u128 << 64;
    let price = sqrt_price_x64_to_price(sqrt_price_x64, 9, 6);

    println!("scenario=orca_whirlpool_token_price");
    println!("program_id={ORCA_WHIRLPOOL_PROGRAM_ID}");
    println!("sqrt_price_x64={sqrt_price_x64}");
    println!("price={price}");
}
