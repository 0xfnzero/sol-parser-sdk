//! Replay raw RPC simulation snapshots using sol-parser-sdk.
//! cargo run --example cpmm_creator_fee_replay -- /absolute/path/to/report.json
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    accounts::{parse_account_unified, AccountData},
    core::events::EventMetadata,
    instr::parse_instruction_unified,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Provide a CPMM simulation report path")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    for result in report["results"].as_array().ok_or("Missing results")? {
        let slot = result["simulation_slot"].as_u64().ok_or("Missing slot")?;
        let instruction = &result["instruction"];
        let data = STANDARD
            .decode(instruction["data_base64"].as_str().ok_or("Missing instruction data")?)?;
        let keys = instruction["accounts"]
            .as_array()
            .ok_or("Missing instruction accounts")?
            .iter()
            .map(|a| {
                a.as_str().ok_or("Invalid account")?.parse::<Pubkey>().map_err(|_| "Invalid pubkey")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let program = instruction["program_id"].as_str().ok_or("Missing program")?.parse()?;
        let event = parse_instruction_unified(
            &data,
            &keys,
            Signature::default(),
            slot,
            0,
            None,
            0,
            None,
            &program,
        )
        .ok_or("Instruction parsing failed")?;
        println!("{}", serde_json::to_string(&event)?);
        for (phase, index) in
            [("before_accounts", 1), ("before_accounts", 2), ("simulated_accounts", 0)]
        {
            let snapshot = &result[phase][index];
            if snapshot.is_null() {
                continue;
            }
            let account = AccountData {
                pubkey: snapshot["pubkey"].as_str().ok_or("Missing pubkey")?.parse()?,
                owner: snapshot["owner"].as_str().ok_or("Missing owner")?.parse()?,
                data: STANDARD.decode(snapshot["data_base64"].as_str().ok_or("Missing data")?)?,
                lamports: snapshot["lamports"].as_u64().ok_or("Missing lamports")?,
                executable: snapshot["executable"].as_bool().ok_or("Missing executable")?,
                rent_epoch: snapshot["rent_epoch"].as_u64().ok_or("Missing rent epoch")?,
            };
            if account.data.is_empty() {
                continue;
            }
            let event = parse_account_unified(&account, EventMetadata::default(), None)
                .ok_or("Account parsing failed")?;
            println!("{}", serde_json::to_string(&event)?);
        }
    }
    Ok(())
}
