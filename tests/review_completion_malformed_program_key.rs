//! Direct protobuf callers must not lose unresolved program/CPI evidence.
//! Malformed protobuf key bytes are not canonical signed transaction wires.
use sol_parser_sdk::analyze_yellowstone_transaction_routes;
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, Transaction,
    TransactionStatusMeta,
};

#[test]
fn malformed_static_and_loaded_program_keys_remain_unknown() {
    let token: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap();
    for position in ["static", "loaded_writable", "loaded_readonly"] {
        for length in [31, 33] {
            let mut keys = vec![
                Pubkey::new_from_array([121; 32]).to_bytes().to_vec(),
                token.to_bytes().to_vec(),
                Pubkey::new_from_array([122; 32]).to_bytes().to_vec(),
                Pubkey::new_from_array([123; 32]).to_bytes().to_vec(),
            ];
            let mut meta = TransactionStatusMeta::default();
            let malformed = vec![0; length];
            let program_index = match position {
                "static" => {
                    keys.push(malformed);
                    4
                }
                "loaded_writable" => {
                    meta.loaded_writable_addresses.push(malformed);
                    4
                }
                _ => {
                    meta.loaded_writable_addresses
                        .push(Pubkey::new_from_array([124; 32]).to_bytes().to_vec());
                    meta.loaded_readonly_addresses.push(malformed);
                    5
                }
            };
            let tx = Transaction {
                message: Some(Message {
                    account_keys: keys,
                    instructions: vec![CompiledInstruction {
                        program_id_index: program_index,
                        accounts: vec![2, 3],
                        data: vec![1],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let mut data = vec![3];
            data.extend_from_slice(&7u64.to_le_bytes());
            meta.inner_instructions.push(InnerInstructions {
                index: 0,
                instructions: vec![InnerInstruction {
                    program_id_index: 1,
                    accounts: vec![2, 3, 0],
                    data,
                    stack_height: Some(2),
                }],
            });
            let route = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
            assert!(route.legs.is_empty());
            assert_eq!(route.transfers.len(), 1);
            assert_eq!(route.unknown_invocations.len(), 1, "{position}/{length}");
            assert_eq!(route.unknown_invocations[0].program, Pubkey::default());
            assert!(route.unknown_invocations[0].has_token_transfers);
            assert!(route.native_token_actions.is_empty());
            // A genuine, correctly sized SystemProgram key remains skippable.
            let mut valid_tx = tx.clone();
            let mut valid_meta = meta.clone();
            match position {
                "static" => valid_tx.message.as_mut().unwrap().account_keys[4] = vec![0; 32],
                "loaded_writable" => valid_meta.loaded_writable_addresses[0] = vec![0; 32],
                _ => valid_meta.loaded_readonly_addresses[0] = vec![0; 32],
            }
            assert!(analyze_yellowstone_transaction_routes(&valid_tx, &valid_meta, &[])
                .unknown_invocations
                .is_empty());
        }
    }
}
