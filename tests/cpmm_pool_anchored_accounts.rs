//! Synthetic router transactions: CPMM swap accounts come from the event's own pool.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    core::events::*,
    grpc::{parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency},
    instr::raydium_cpmm as cpmm,
    DexEvent,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, SubscribeUpdateTransaction,
    SubscribeUpdateTransactionInfo, Transaction, TransactionStatusMeta,
};

/// Swap instruction accounts; index 3 is the pool.
fn swap_accounts() -> Vec<Pubkey> {
    let mut a: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
    a[8] = spl_token::id();
    a[9] = spl_token_2022::id();
    a
}

fn swap_instruction() -> Vec<u8> {
    let mut data = cpmm::discriminators::SWAP_BASE_IN.to_vec();
    data.extend(999_999_u64.to_le_bytes());
    data.extend(777_777_u64.to_le_bytes());
    data
}

fn swap_log(a: &[Pubkey], amount: u64) -> String {
    let mut data = sol_parser_sdk::logs::raydium_cpmm::discriminators::SWAP_EVENT.to_vec();
    data.extend(a[3].to_bytes());
    for value in [1000, 2000, amount, amount * 2, 3, 4] {
        data.extend(value.to_le_bytes());
    }
    data.push(1);
    format!("Program data: {}", STANDARD.encode(data))
}

/// Router outer instruction with two inner CPMM swaps: `a` then `b`.
fn router_tx(a: &[Pubkey], b: &[Pubkey], logs: [String; 2]) -> SubscribeUpdateTransaction {
    let router = Pubkey::new_unique();
    let message = Message {
        account_keys: vec![router.to_bytes().to_vec(), cpmm::PROGRAM_ID_PUBKEY.to_bytes().to_vec()],
        instructions: vec![CompiledInstruction { program_id_index: 0, ..Default::default() }],
        ..Default::default()
    };
    let inner = |accounts: std::ops::Range<u8>| InnerInstruction {
        program_id_index: 1,
        accounts: accounts.collect(),
        data: swap_instruction(),
        stack_height: Some(2),
    };
    let [log_a, log_b] = logs;
    let meta = TransactionStatusMeta {
        loaded_writable_addresses: a.iter().map(|p| p.to_bytes().to_vec()).collect(),
        loaded_readonly_addresses: b.iter().map(|p| p.to_bytes().to_vec()).collect(),
        inner_instructions: vec![InnerInstructions {
            index: 0,
            instructions: vec![inner(2..15), inner(15..28)],
        }],
        log_messages: vec![
            format!("Program {router} invoke [1]"),
            format!("Program {} invoke [2]", cpmm::PROGRAM_ID_PUBKEY),
            log_a,
            format!("Program {} success", cpmm::PROGRAM_ID_PUBKEY),
            format!("Program {} invoke [2]", cpmm::PROGRAM_ID_PUBKEY),
            log_b,
            format!("Program {} success", cpmm::PROGRAM_ID_PUBKEY),
            format!("Program {router} success"),
        ],
        ..Default::default()
    };
    SubscribeUpdateTransaction {
        slot: 42,
        transaction: Some(SubscribeUpdateTransactionInfo {
            signature: Signature::default().as_ref().to_vec(),
            transaction: Some(Transaction { message: Some(message), ..Default::default() }),
            meta: Some(meta),
            ..Default::default()
        }),
    }
}

fn swaps(tx: &SubscribeUpdateTransaction) -> [Vec<RaydiumCpmmSwapEvent>; 2] {
    [
        parse_subscribe_update_transaction(tx, 0, None, None),
        parse_subscribe_update_transaction_low_latency(tx, 0, None, None),
    ]
    .map(|events| {
        events
            .into_iter()
            .filter_map(|e| if let DexEvent::RaydiumCpmmSwap(s) = e { Some(s) } else { None })
            .collect()
    })
}

fn assert_accounts_of(s: &RaydiumCpmmSwapEvent, a: &[Pubkey]) {
    assert_eq!(s.payer, a[0]);
    assert_eq!(s.amm_config, a[2]);
    assert_eq!((s.input_vault, s.output_vault), (a[6], a[7]));
    assert_eq!((s.input_token_program, s.output_token_program), (a[8], a[9]));
    assert_eq!((s.input_token_mint, s.output_token_mint), (a[10], a[11]));
    assert_eq!(s.observation_state, a[12]);
}

#[test]
fn two_pool_route_fills_each_swap_from_its_own_pool() {
    let (a, b) = (swap_accounts(), swap_accounts());
    let tx = router_tx(&a, &b, [swap_log(&a, 10), swap_log(&b, 20)]);
    for events in swaps(&tx) {
        for (pool, accounts) in [(a[3], &a), (b[3], &b)] {
            let logged = events.iter().filter(|s| s.pool_id == pool && s.input_amount > 0);
            let mut logged = logged.peekable();
            assert!(logged.peek().is_some());
            logged.for_each(|s| assert_accounts_of(s, accounts));
        }
    }
}

#[test]
fn swap_log_of_a_pool_without_invocation_stays_unfilled() {
    let (a, b, stray) = (swap_accounts(), swap_accounts(), swap_accounts());
    let tx = router_tx(&a, &b, [swap_log(&a, 10), swap_log(&stray, 20)]);
    for events in swaps(&tx) {
        let s = events.iter().find(|s| s.pool_id == stray[3]).unwrap();
        assert_eq!(s.payer, Pubkey::default());
        assert_eq!(s.amm_config, Pubkey::default());
        assert_eq!((s.input_vault, s.output_vault), (Pubkey::default(), Pubkey::default()));
        assert_eq!(s.input_token_mint, Pubkey::default());
    }
}
