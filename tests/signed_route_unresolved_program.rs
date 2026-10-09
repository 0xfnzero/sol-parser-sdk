//! A signature does not make an out-of-range program reference trustworthy.
use sol_parser_sdk::analyze_yellowstone_transaction_routes;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    message::{VersionedMessage, Message as WireMessage},
    transaction::VersionedTransaction,
    message::compiled_instruction::CompiledInstruction as WireInstruction,
};
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, Transaction,
    TransactionStatusMeta,
};

#[test]
fn independently_signed_unresolved_program_stays_unknown_with_cpi_evidence() {
    let payer = Keypair::new_from_array([113; 32]);
    let token = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse::<Pubkey>().unwrap();
    let source = Pubkey::new_from_array([114; 32]);
    let destination = Pubkey::new_from_array([115; 32]);
    let mut message = WireMessage::new(&[], Some(&payer.pubkey()));
    message.account_keys.extend([token, source, destination]);
    message.instructions.push(WireInstruction {
        program_id_index: 255,
        accounts: vec![2, 3],
        data: vec![1],
    });
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&payer]).unwrap();
    assert_eq!(tx.signatures.len(), 1);
    assert!(tx.signatures[0].verify(payer.pubkey().as_ref(), &tx.message.serialize()));
    assert!(tx.sanitize().is_err());
    let decoded: VersionedTransaction =
        wincode::deserialize_exact(&wincode::serialize(&tx).unwrap()).unwrap();
    let projection = Transaction {
        signatures: decoded
            .signatures
            .iter()
            .map(|s| s.as_ref().to_vec())
            .collect(),
        message: Some(Message {
            account_keys: decoded
                .message
                .static_account_keys()
                .iter()
                .map(|k| k.to_bytes().to_vec())
                .collect(),
            instructions: decoded
                .message
                .instructions()
                .iter()
                .map(|ix| CompiledInstruction {
                    program_id_index: u32::from(ix.program_id_index),
                    accounts: ix.accounts.clone(),
                    data: ix.data.clone(),
                })
                .collect(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut transfer_data = vec![3];
    transfer_data.extend_from_slice(&7u64.to_le_bytes());
    let meta = TransactionStatusMeta {
        inner_instructions: vec![InnerInstructions {
            index: 0,
            instructions: vec![InnerInstruction {
                program_id_index: 1,
                accounts: vec![2, 3, 0],
                data: transfer_data,
                stack_height: Some(2),
            }],
        }],
        ..Default::default()
    };
    let route = analyze_yellowstone_transaction_routes(&projection, &meta, &[]);
    assert!(route.legs.is_empty());
    assert_eq!(route.signature, decoded.signatures[0]);
    assert_eq!(route.transfers.len(), 1);
    assert_eq!(route.unknown_invocations.len(), 1);
    assert_eq!(route.unknown_invocations[0].position.outer_index, 0);
    assert_eq!(route.unknown_invocations[0].position.inner_index, None);
    assert_eq!(route.unknown_invocations[0].program, Pubkey::default());
    assert!(route.unknown_invocations[0].has_token_transfers);
    assert!(!route.unknown_invocations[0].has_known_swap_descendants);
}
