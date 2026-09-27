//! Offline parser contract: the saved fixture is generic CPMM, not StonkFun evidence.
use sol_parser_sdk::{convert_rpc_to_grpc, parse_rpc_transaction, DexEvent};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

#[test]
fn saved_cpmm_multileg_exposes_actual_invocation_actor() {
    let tx: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(include_str!("fixtures/raydium_cpmm_rpc_transaction.json")).unwrap();
    let (meta, grpc) = convert_rpc_to_grpc(&tx).unwrap();
    let message = grpc.message.as_ref().unwrap();
    let keys: Vec<_> = message
        .account_keys
        .iter()
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .map(|bytes| Pubkey::try_from(bytes.as_slice()).unwrap())
        .collect();
    let cpmm = sol_parser_sdk::instr::raydium_cpmm::PROGRAM_ID_PUBKEY;
    let actors: Vec<_> = meta
        .inner_instructions
        .iter()
        .flat_map(|g| &g.instructions)
        .filter(|ix| keys[ix.program_id_index as usize] == cpmm && ix.accounts.len() >= 13)
        .map(|ix| keys[ix.accounts[0] as usize])
        .collect();
    let swaps: Vec<_> = parse_rpc_transaction(&tx, None)
        .unwrap()
        .into_iter()
        .filter_map(|e| if let DexEvent::RaydiumCpmmSwap(s) = e { Some(s) } else { None })
        .collect();
    assert_eq!(actors.len(), 3);
    assert_eq!(swaps.len(), 3);
    for (swap, actor) in swaps.iter().zip(actors) {
        let value = serde_json::to_value(swap).unwrap();
        assert_eq!(
            value["context"]["payer"],
            serde_json::to_value(actor).unwrap(),
            "executed CPMM event must expose its own instruction actor"
        );
    }
    let update = yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction {
        slot: tx.slot,
        transaction: Some(yellowstone_grpc_proto::prelude::SubscribeUpdateTransactionInfo {
            signature: grpc.signatures[0].clone(),
            transaction: Some(grpc),
            meta: Some(meta),
            ..Default::default()
        }),
    };
    for events in [
        sol_parser_sdk::grpc::parse_subscribe_update_transaction(&update, 0, None, None),
        sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update, 0, None, None,
        ),
    ] {
        let stream: Vec<_> = events
            .into_iter()
            .filter_map(|e| if let DexEvent::RaydiumCpmmSwap(s) = e { Some(s) } else { None })
            .collect();
        assert_eq!(stream.len(), 3);
        for (rpc, stream) in swaps.iter().zip(stream) {
            assert_eq!(rpc.context, stream.context);
            assert_eq!(rpc.instruction_index, stream.instruction_index);
            assert_eq!(rpc.log_mints, stream.log_mints);
            assert_eq!(
                (rpc.input_amount, rpc.output_amount),
                (stream.input_amount, stream.output_amount)
            );
            assert_eq!(stream.transaction_success, Some(true));
        }
    }
}

fn migration_accounts() -> Vec<Pubkey> {
    let mut accounts: Vec<_> = (0..28).map(|_| Pubkey::new_unique()).collect();
    accounts[4] = sol_parser_sdk::instr::raydium_cpmm::PROGRAM_ID_PUBKEY;
    accounts[22] = spl_token_2022::id();
    accounts[23] = spl_token::id();
    accounts[4] = solana_sdk::pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    accounts[13] = solana_sdk::pubkey!("LockrWmn6K5twhz3y9w1dQERbmgSaRkfnTeTKbpofwE");
    accounts[23] = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    accounts[24] = solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    accounts[25] = solana_sdk::pubkey!("11111111111111111111111111111111");
    accounts[26] = solana_sdk::pubkey!("SysvarRent111111111111111111111111111111111");
    accounts[27] = solana_sdk::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
    accounts
}

#[test]
fn launchlab_cpswap_migration_emits_linkage() {
    let accounts = migration_accounts();
    let event = sol_parser_sdk::instr::raydium_launchlab::parse_instruction(
        &sol_parser_sdk::instr::raydium_launchlab::discriminators::MIGRATE_TO_CPSWAP,
        &accounts,
        Signature::default(),
        1,
        0,
        None,
    );
    assert!(event.is_some(), "IDL migrate_to_cpswap must emit actual old/new pool linkage");
}
