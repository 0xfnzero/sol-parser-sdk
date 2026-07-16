use sol_parser_sdk::grpc::program_ids::METEORA_DAMM_V2_PROGRAM_ID;
use sol_parser_sdk::grpc::Protocol;

fn main() {
    let protocol = Protocol::MeteoraDammV2;

    println!("scenario=meteora_damm_new_pool");
    println!("protocol={protocol:?}");
    println!("program_id={METEORA_DAMM_V2_PROGRAM_ID}");
    println!("new_pool_event=EvtInitializePool");
}
