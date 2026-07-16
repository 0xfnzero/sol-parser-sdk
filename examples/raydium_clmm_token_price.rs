use sol_parser_sdk::grpc::program_ids::RAYDIUM_CLMM_PROGRAM_ID;
use sol_parser_sdk::utils::sqrt_price_x64_to_price;

fn main() {
    let sqrt_price_x64 = 1_u128 << 64;
    let price = sqrt_price_x64_to_price(sqrt_price_x64, 6, 6);

    println!("scenario=raydium_clmm_token_price");
    println!("program_id={RAYDIUM_CLMM_PROGRAM_ID}");
    println!("sqrt_price_x64={sqrt_price_x64}");
    println!("price={price}");
}
