use sol_parser_sdk::grpc::program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter, Protocol};

fn main() {
    let protocol = Protocol::RaydiumLaunchlab;
    let filter = EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabMigrateAmm]);

    println!("scenario=raydium_launchlab_migration");
    println!("protocol={protocol:?}");
    println!("program_id={RAYDIUM_LAUNCHLAB_PROGRAM_ID}");
    println!("filter_includes_launchlab={}", filter.includes_raydium_launchlab());
}
