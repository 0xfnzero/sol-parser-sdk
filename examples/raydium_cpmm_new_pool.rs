use sol_parser_sdk::grpc::program_ids::RAYDIUM_CPMM_PROGRAM_ID;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter, Protocol};

fn main() {
    let protocol = Protocol::RaydiumCpmm;
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmInitialize]);

    println!("scenario=raydium_cpmm_new_pool");
    println!("protocol={protocol:?}");
    println!("program_id={RAYDIUM_CPMM_PROGRAM_ID}");
    println!("filter_includes_cpmm={}", filter.includes_raydium_cpmm());
}
