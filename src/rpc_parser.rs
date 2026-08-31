//! RPC Transaction Parser
//!
//! 提供独立的 RPC 交易解析功能，不依赖 gRPC streaming
//! 可以用于测试验证和离线分析

use crate::core::events::DexEvent;
use crate::grpc::types::EventTypeFilter;
use crate::instr::read_pubkey_fast;
use crate::transaction_cost::{parse_yellowstone_transaction_cost, TransactionCost};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionConfig, UiTransactionEncoding};
use solana_client::rpc_response::{EncodedTransaction, UiInstruction, UiTransactionTokenBalance};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, MessageAddressTableLookup,
    MessageHeader, TokenBalance, Transaction, TransactionError, TransactionStatusMeta,
    UiTokenAmount,
};

/// Parse a transaction from RPC by signature
///
/// # Arguments
/// * `rpc_client` - RPC client to fetch the transaction
/// * `signature` - Transaction signature
/// * `filter` - Optional event type filter
///
/// # Returns
/// Vector of parsed DEX events
///
/// # Example
/// ```no_run
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::signature::Signature;
/// use sol_parser_sdk::parse_transaction_from_rpc;
/// use std::str::FromStr;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let sig = Signature::from_str("your-signature-here").unwrap();
/// let events = parse_transaction_from_rpc(&client, &sig, None).unwrap();
/// ```
pub fn parse_transaction_from_rpc(
    rpc_client: &RpcClient,
    signature: &Signature,
    filter: Option<&EventTypeFilter>,
) -> Result<Vec<DexEvent>, ParseError> {
    // Fetch transaction from RPC with V1 transaction support.
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: None,
        max_supported_transaction_version: Some(1),
    };

    let rpc_tx = rpc_client.get_transaction_with_config(signature, config).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("invalid type: null") && msg.contains("EncodedConfirmedTransactionWithStatusMeta") {
            ParseError::RpcError(format!(
                "Transaction not found (RPC returned null). Common causes: 1) Transaction is too old and pruned (use an archive RPC). 2) Wrong network or invalid signature. Try SOLANA_RPC_URL with an archive endpoint (e.g. Helius, QuickNode) or a more recent tx. Original: {}",
                msg
            ))
        } else {
            ParseError::RpcError(msg)
        }
    })?;

    parse_rpc_transaction(&rpc_tx, filter)
}

/// Parse a RPC transaction structure
///
/// # Arguments
/// * `rpc_tx` - RPC transaction to parse
/// * `filter` - Optional event type filter
///
/// # Returns
/// Vector of parsed DEX events
///
/// # Example
/// ```no_run
/// use sol_parser_sdk::parse_rpc_transaction;
///
/// // Assuming you have an rpc_tx from RPC
/// // let events = parse_rpc_transaction(&rpc_tx, None).unwrap();
/// ```
pub fn parse_rpc_transaction(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
    filter: Option<&EventTypeFilter>,
) -> Result<Vec<DexEvent>, ParseError> {
    let (grpc_meta, grpc_tx) = convert_rpc_to_grpc(rpc_tx)?;
    let signature = extract_grpc_signature(&grpc_tx)?;
    parse_converted_rpc_transaction(rpc_tx, grpc_meta, grpc_tx, signature, filter)
}

/// Result of parsing RPC events and transaction costs from one shared decode.
#[derive(Debug)]
pub struct ParsedRpcTransaction {
    pub events: Vec<DexEvent>,
    pub cost: TransactionCost,
    pub signature: Signature,
}

/// Parses events and transaction costs while decoding the RPC payload once.
pub fn parse_rpc_transaction_with_cost(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
    filter: Option<&EventTypeFilter>,
) -> Result<ParsedRpcTransaction, ParseError> {
    let (grpc_meta, grpc_tx) = convert_rpc_to_grpc(rpc_tx)?;
    let signature = extract_grpc_signature(&grpc_tx)?;
    let cost = parse_yellowstone_transaction_cost(&grpc_tx, &grpc_meta)
        .ok_or_else(|| ParseError::MissingField("transaction.message".to_string()))?;
    let events = parse_converted_rpc_transaction(rpc_tx, grpc_meta, grpc_tx, signature, filter)?;
    Ok(ParsedRpcTransaction { events, cost, signature })
}

/// Parses only transaction cost and signature from one shared RPC decode.
pub fn parse_rpc_transaction_cost_with_signature(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<(TransactionCost, Signature), ParseError> {
    let (grpc_meta, grpc_tx) = convert_rpc_to_grpc(rpc_tx)?;
    let signature = extract_grpc_signature(&grpc_tx)?;
    let cost = parse_yellowstone_transaction_cost(&grpc_tx, &grpc_meta)
        .ok_or_else(|| ParseError::MissingField("transaction.message".to_string()))?;
    Ok((cost, signature))
}

fn extract_grpc_signature(transaction: &Transaction) -> Result<Signature, ParseError> {
    transaction
        .signatures
        .first()
        .ok_or_else(|| ParseError::MissingField("transaction.signatures[0]".to_string()))
        .and_then(|bytes| {
            Signature::try_from(bytes.as_slice()).map_err(|error| {
                ParseError::ConversionError(format!("Invalid transaction signature: {error}"))
            })
        })
}

fn parse_converted_rpc_transaction(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
    grpc_meta: TransactionStatusMeta,
    grpc_tx: Transaction,
    signature: Signature,
    filter: Option<&EventTypeFilter>,
) -> Result<Vec<DexEvent>, ParseError> {
    // Extract metadata
    let slot = rpc_tx.slot;
    let block_time_us = rpc_tx.block_time.map(|t| t * 1_000_000);
    let grpc_recv_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64;

    // Wrap grpc_tx in Option for reuse
    let grpc_tx_opt = Some(grpc_tx);

    let mut program_invokes: HashMap<Pubkey, Vec<(i32, i32)>> = HashMap::new();

    if let Some(ref tx) = grpc_tx_opt {
        if let Some(ref msg) = tx.message {
            let keys_len = msg.account_keys.len();
            let writable_len = grpc_meta.loaded_writable_addresses.len();
            let get_key = |i: usize| -> Option<&Vec<u8>> {
                if i < keys_len {
                    msg.account_keys.get(i)
                } else if i < keys_len + writable_len {
                    grpc_meta.loaded_writable_addresses.get(i - keys_len)
                } else {
                    grpc_meta.loaded_readonly_addresses.get(i - keys_len - writable_len)
                }
            };

            for (i, ix) in msg.instructions.iter().enumerate() {
                let pid = get_key(ix.program_id_index as usize)
                    .map_or(Pubkey::default(), |k| read_pubkey_fast(k));
                if crate::grpc::program_ids::needs_invoke_context(&pid) {
                    program_invokes.entry(pid).or_default().push((i as i32, -1));
                }
            }

            for inner in &grpc_meta.inner_instructions {
                let outer_idx = inner.index as usize;
                for (j, inner_ix) in inner.instructions.iter().enumerate() {
                    let pid = get_key(inner_ix.program_id_index as usize)
                        .map_or(Pubkey::default(), |k| read_pubkey_fast(k));
                    if crate::grpc::program_ids::needs_invoke_context(&pid) {
                        program_invokes.entry(pid).or_default().push((outer_idx as i32, j as i32));
                    }
                }
            }
        }
    }

    let needs_pumpfun = filter.map(EventTypeFilter::includes_pumpfun).unwrap_or(true);
    let is_created_buy = needs_pumpfun
        && crate::logs::optimized_matcher::detect_pumpfun_create(&grpc_meta.log_messages);

    // Parse instructions
    let instr_events =
        crate::grpc::instruction_parser::parse_instructions_enhanced_with_created_buy(
            &grpc_meta,
            &grpc_tx_opt,
            signature,
            slot,
            0, // tx_idx
            block_time_us,
            grpc_recv_us,
            filter,
            is_created_buy,
        );

    // Parse logs (for protocols like PumpFun that emit events in logs)
    struct ActiveProgram<'a> {
        encoded: &'a str,
        pubkey: Pubkey,
    }

    let mut active_program_stack: Vec<ActiveProgram<'_>> = Vec::with_capacity(8);
    let mut log_events = Vec::new();

    for log in &grpc_meta.log_messages {
        if let Some((pid, depth)) = crate::logs::optimized_matcher::parse_invoke_info(log) {
            let pk = crate::grpc::program_ids::known_program_id(pid).unwrap_or_default();
            active_program_stack.truncate(depth - 1);
            active_program_stack.push(ActiveProgram { encoded: pid, pubkey: pk });
        }

        if let Some(mut event) = crate::logs::parse_log_with_program_id(
            log,
            signature,
            slot,
            0, // tx_index
            block_time_us,
            grpc_recv_us,
            filter,
            is_created_buy,
            None,
            active_program_stack.last().map(|active| &active.pubkey),
        ) {
            // Fill account fields - use same function as gRPC parsing
            crate::core::account_dispatcher::fill_accounts_with_owned_keys(
                &mut event,
                &grpc_meta,
                &grpc_tx_opt,
                &program_invokes,
            );

            // Fill additional data fields (e.g., PumpSwap is_pump_pool)
            crate::core::common_filler::fill_data(
                &mut event,
                &grpc_meta,
                &grpc_tx_opt,
                &program_invokes,
            );

            log_events.push(event);
        }

        if let Some(pid) = crate::logs::optimized_matcher::parse_program_complete_info(log) {
            if let Some(pos) = active_program_stack.iter().rposition(|active| active.encoded == pid)
            {
                active_program_stack.truncate(pos);
            }
        }
    }

    let mut events = merge_log_and_instruction_events(log_events, instr_events);
    fill_rpc_event_metadata(&mut events, &grpc_meta, &grpc_tx_opt);
    Ok(events)
}

fn fill_rpc_event_metadata(
    events: &mut [DexEvent],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
) {
    for event in events.iter_mut() {
        crate::core::common_filler::fill_token_balances(event, meta, transaction);
    }
    crate::grpc::transaction_meta::fill_recent_blockhash(events, transaction);
}

fn merge_log_and_instruction_events(
    log_events: Vec<DexEvent>,
    instr_events: Vec<DexEvent>,
) -> Vec<DexEvent> {
    crate::grpc::log_instr_dedup::dedupe_log_instruction_events(log_events, instr_events)
}

/// Parse error types
#[derive(Debug)]
pub enum ParseError {
    RpcError(String),
    ConversionError(String),
    MissingField(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            ParseError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
            ParseError::MissingField(msg) => write!(f, "Missing field: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

// ============================================================================
// Internal conversion functions
// ============================================================================

pub fn convert_rpc_to_grpc(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<(TransactionStatusMeta, Transaction), ParseError> {
    let rpc_meta = rpc_tx
        .transaction
        .meta
        .as_ref()
        .ok_or_else(|| ParseError::MissingField("meta".to_string()))?;

    let (loaded_writable_addresses, loaded_readonly_addresses) = rpc_meta
        .loaded_addresses
        .as_ref()
        .map(|addresses| {
            let writable = addresses
                .writable
                .iter()
                .map(|address| parse_loaded_address(address))
                .collect::<Result<Vec<_>, _>>()?;
            let readonly = addresses
                .readonly
                .iter()
                .map(|address| parse_loaded_address(address))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((writable, readonly))
        })
        .transpose()?
        .unwrap_or_default();

    let err = rpc_meta
        .err
        .clone()
        .map(|error| {
            let error: solana_sdk::transaction::TransactionError = error.into();
            wincode::serialize(&error).map(|err| TransactionError { err }).map_err(|error| {
                ParseError::ConversionError(format!(
                    "Failed to serialize transaction error: {error}"
                ))
            })
        })
        .transpose()?;

    // Convert meta
    let mut grpc_meta = TransactionStatusMeta {
        err,
        fee: rpc_meta.fee,
        pre_balances: rpc_meta.pre_balances.clone(),
        post_balances: rpc_meta.post_balances.clone(),
        inner_instructions: Vec::new(),
        log_messages: rpc_meta
            .log_messages
            .as_ref()
            .map(|messages| messages.clone())
            .unwrap_or_default(),
        pre_token_balances: rpc_meta
            .pre_token_balances
            .as_ref()
            .map(|balances| convert_token_balances(balances))
            .unwrap_or_default(),
        post_token_balances: rpc_meta
            .post_token_balances
            .as_ref()
            .map(|balances| convert_token_balances(balances))
            .unwrap_or_default(),
        rewards: Vec::new(),
        loaded_writable_addresses,
        loaded_readonly_addresses,
        return_data: None,
        compute_units_consumed: rpc_meta.compute_units_consumed.clone().into(),

        inner_instructions_none: !rpc_meta.inner_instructions.is_some(),
        log_messages_none: !rpc_meta.log_messages.is_some(),
        return_data_none: !rpc_meta.return_data.is_some(),
        cost_units: rpc_meta.cost_units.clone().into(),
    };

    // Convert inner instructions
    if let solana_transaction_status::option_serializer::OptionSerializer::Some(
        inner_instructions,
    ) = rpc_meta.inner_instructions.as_ref()
    {
        for inner in inner_instructions {
            let mut grpc_inner =
                InnerInstructions { index: inner.index as u32, instructions: Vec::new() };

            for ix in &inner.instructions {
                if let UiInstruction::Compiled(compiled) = ix {
                    // Decode base58 data
                    let data = bs58::decode(&compiled.data).into_vec().map_err(|e| {
                        ParseError::ConversionError(format!(
                            "Failed to decode instruction data: {}",
                            e
                        ))
                    })?;

                    grpc_inner.instructions.push(InnerInstruction {
                        program_id_index: compiled.program_id_index as u32,
                        accounts: compiled.accounts.clone(),
                        data,
                        stack_height: compiled.stack_height,
                    });
                }
            }

            grpc_meta.inner_instructions.push(grpc_inner);
        }
    }

    // Convert transaction
    let ui_tx = &rpc_tx.transaction.transaction;

    let (message, signatures) = match ui_tx {
        EncodedTransaction::Binary(_, _) | EncodedTransaction::LegacyBinary(_) => {
            // Solana's decoder handles Base58/Base64, wincode deserialization and sanitization.
            let versioned_tx = ui_tx.decode().ok_or_else(|| {
                ParseError::ConversionError(
                    "Failed to decode or sanitize binary transaction".to_string(),
                )
            })?;

            let sigs: Vec<Vec<u8>> =
                versioned_tx.signatures.iter().map(|s| s.as_ref().to_vec()).collect();

            let message = match versioned_tx.message {
                solana_sdk::message::VersionedMessage::Legacy(legacy_msg) => {
                    convert_legacy_message(legacy_msg)?
                }
                solana_sdk::message::VersionedMessage::V0(v0_msg) => convert_v0_message(v0_msg)?,
                solana_sdk::message::VersionedMessage::V1(v1_msg) => convert_v1_message(v1_msg)?,
            };

            (message, sigs)
        }
        EncodedTransaction::Json(_) => {
            return Err(ParseError::ConversionError(
                "JSON encoded transactions not supported yet".to_string(),
            ));
        }
        _ => {
            return Err(ParseError::ConversionError(
                "Unsupported transaction encoding".to_string(),
            ));
        }
    };

    let grpc_tx = Transaction { signatures, message: Some(message) };

    Ok((grpc_meta, grpc_tx))
}

fn parse_loaded_address(address: &str) -> Result<Vec<u8>, ParseError> {
    address.parse::<Pubkey>().map(|pubkey| pubkey.to_bytes().to_vec()).map_err(|error| {
        ParseError::ConversionError(format!("Invalid loaded address {address}: {error}"))
    })
}

fn convert_token_balances(balances: &[UiTransactionTokenBalance]) -> Vec<TokenBalance> {
    balances
        .iter()
        .map(|balance| TokenBalance {
            account_index: balance.account_index as u32,
            mint: balance.mint.clone(),
            ui_token_amount: Some(UiTokenAmount {
                ui_amount: balance.ui_token_amount.ui_amount.unwrap_or_default(),
                decimals: balance.ui_token_amount.decimals as u32,
                amount: balance.ui_token_amount.amount.clone(),
                ui_amount_string: balance.ui_token_amount.ui_amount_string.clone(),
            }),
            owner: balance.owner.as_ref().map(|owner| owner.clone()).unwrap_or_default(),
            program_id: balance
                .program_id
                .as_ref()
                .map(|program_id| program_id.clone())
                .unwrap_or_default(),
        })
        .collect()
}

fn convert_legacy_message(
    msg: solana_sdk::message::legacy::Message,
) -> Result<Message, ParseError> {
    let account_keys: Vec<Vec<u8>> =
        msg.account_keys.iter().map(|k| k.to_bytes().to_vec()).collect();

    let instructions: Vec<CompiledInstruction> = msg
        .instructions
        .into_iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u32,
            accounts: ix.accounts,
            data: ix.data,
        })
        .collect();

    Ok(Message {
        header: Some(MessageHeader {
            num_required_signatures: msg.header.num_required_signatures as u32,
            num_readonly_signed_accounts: msg.header.num_readonly_signed_accounts as u32,
            num_readonly_unsigned_accounts: msg.header.num_readonly_unsigned_accounts as u32,
        }),
        account_keys,
        recent_blockhash: msg.recent_blockhash.to_bytes().to_vec(),
        instructions,
        versioned: false,
        address_table_lookups: Vec::new(),
        config: None,
    })
}

fn convert_v0_message(msg: solana_sdk::message::v0::Message) -> Result<Message, ParseError> {
    let account_keys: Vec<Vec<u8>> =
        msg.account_keys.iter().map(|k| k.to_bytes().to_vec()).collect();

    let instructions: Vec<CompiledInstruction> = msg
        .instructions
        .into_iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u32,
            accounts: ix.accounts,
            data: ix.data,
        })
        .collect();

    Ok(Message {
        header: Some(MessageHeader {
            num_required_signatures: msg.header.num_required_signatures as u32,
            num_readonly_signed_accounts: msg.header.num_readonly_signed_accounts as u32,
            num_readonly_unsigned_accounts: msg.header.num_readonly_unsigned_accounts as u32,
        }),
        account_keys,
        recent_blockhash: msg.recent_blockhash.to_bytes().to_vec(),
        instructions,
        versioned: true,
        address_table_lookups: msg
            .address_table_lookups
            .into_iter()
            .map(|lookup| MessageAddressTableLookup {
                account_key: lookup.account_key.to_bytes().to_vec(),
                writable_indexes: lookup.writable_indexes,
                readonly_indexes: lookup.readonly_indexes,
            })
            .collect(),
        config: None,
    })
}

fn convert_v1_message(msg: solana_sdk::message::v1::Message) -> Result<Message, ParseError> {
    let account_keys = msg.account_keys.iter().map(|key| key.to_bytes().to_vec()).collect();
    let instructions = msg
        .instructions
        .into_iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u32,
            accounts: ix.accounts,
            data: ix.data,
        })
        .collect();

    Ok(Message {
        header: Some(MessageHeader {
            num_required_signatures: msg.header.num_required_signatures as u32,
            num_readonly_signed_accounts: msg.header.num_readonly_signed_accounts as u32,
            num_readonly_unsigned_accounts: msg.header.num_readonly_unsigned_accounts as u32,
        }),
        account_keys,
        recent_blockhash: msg.lifetime_specifier.to_bytes().to_vec(),
        instructions,
        versioned: true,
        address_table_lookups: Vec::new(),
        config: Some(yellowstone_grpc_proto::prelude::TransactionConfig {
            priority_fee: msg.config.priority_fee,
            compute_unit_limit: msg.config.compute_unit_limit,
            loaded_accounts_data_size_limit: msg.config.loaded_accounts_data_size_limit,
            heap_size: msg.config.heap_size,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{
        DexEvent, EventMetadata, PumpFunTradeEvent, PumpSwapCreatePoolEvent,
    };
    use base64::{engine::general_purpose, Engine as _};
    use solana_client::rpc_response::{
        UiLoadedAddresses, UiTokenAmount as RpcUiTokenAmount, UiTransactionStatusMeta,
        UiTransactionTokenBalance,
    };
    use solana_sdk::{
        hash::Hash,
        message::{legacy, MessageHeader, VersionedMessage},
        pubkey::Pubkey,
        signature::Signature,
        transaction::VersionedTransaction,
    };
    use solana_transaction_status::{
        option_serializer::OptionSerializer, EncodedTransactionWithStatusMeta,
        TransactionBinaryEncoding,
    };

    fn rpc_fixture(
        user: Pubkey,
        token_account: Pubkey,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let transaction = VersionedTransaction {
            signatures: vec![Signature::from([7; 64])],
            message: VersionedMessage::Legacy(legacy::Message {
                header: MessageHeader { num_required_signatures: 1, ..Default::default() },
                account_keys: vec![user, token_account],
                recent_blockhash: Hash::new_unique(),
                instructions: Vec::new(),
            }),
        };
        let bytes = wincode::serialize(&transaction).expect("serialize RPC fixture");
        let token_balance = |amount: &str| UiTransactionTokenBalance {
            account_index: 1,
            mint: Pubkey::new_unique().to_string(),
            ui_token_amount: RpcUiTokenAmount {
                ui_amount: None,
                decimals: 6,
                amount: amount.to_string(),
                ui_amount_string: amount.to_string(),
            },
            owner: OptionSerializer::Some(user.to_string()),
            program_id: OptionSerializer::None,
        };

        EncodedConfirmedTransactionWithStatusMeta {
            slot: 42,
            transaction: EncodedTransactionWithStatusMeta {
                transaction: EncodedTransaction::Binary(
                    general_purpose::STANDARD.encode(bytes),
                    TransactionBinaryEncoding::Base64,
                ),
                meta: Some(UiTransactionStatusMeta {
                    err: None,
                    status: Ok(()),
                    fee: 5_000,
                    pre_balances: vec![50_000, 2_039_280],
                    post_balances: vec![40_000, 2_039_280],
                    inner_instructions: OptionSerializer::None,
                    log_messages: OptionSerializer::None,
                    pre_token_balances: OptionSerializer::Some(vec![token_balance("10")]),
                    post_token_balances: OptionSerializer::Some(vec![token_balance("35")]),
                    rewards: OptionSerializer::None,
                    loaded_addresses: OptionSerializer::None,
                    return_data: OptionSerializer::None,
                    compute_units_consumed: OptionSerializer::Some(123),
                    cost_units: OptionSerializer::Some(456),
                }),
                version: None,
            },
            block_time: None,
            transaction_index: None,
        }
    }

    fn dummy_meta() -> EventMetadata {
        EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        }
    }

    #[test]
    fn rpc_merge_keeps_instruction_cashback_for_log_only_pumpswap_create_pool() {
        let pool = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let log_create = PumpSwapCreatePoolEvent {
            metadata: dummy_meta(),
            pool,
            base_mint,
            quote_mint,
            is_cashback_coin: false,
            ..Default::default()
        };
        let ix_create = PumpSwapCreatePoolEvent {
            metadata: dummy_meta(),
            pool,
            base_mint,
            quote_mint,
            is_cashback_coin: true,
            ..Default::default()
        };

        let merged = merge_log_and_instruction_events(
            vec![DexEvent::PumpSwapCreatePool(log_create)],
            vec![DexEvent::PumpSwapCreatePool(ix_create)],
        );

        assert_eq!(merged.len(), 1);
        match &merged[0] {
            DexEvent::PumpSwapCreatePool(e) => assert!(e.is_cashback_coin),
            other => panic!("expected PumpSwapCreatePool, got {other:?}"),
        }
    }

    #[test]
    fn rpc_token_balances_fill_pumpfun_trade_without_an_rpc_balance_lookup() {
        let user = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let rpc_tx = rpc_fixture(user, token_account);
        let (meta, transaction) = convert_rpc_to_grpc(&rpc_tx).expect("convert RPC fixture");

        assert_eq!(meta.pre_token_balances[0].ui_token_amount.as_ref().unwrap().amount, "10");
        assert_eq!(meta.post_token_balances[0].ui_token_amount.as_ref().unwrap().amount, "35");
        assert_eq!(meta.compute_units_consumed, Some(123));
        assert_eq!(meta.cost_units, Some(456));

        let mut events = vec![DexEvent::PumpFunTrade(PumpFunTradeEvent {
            user,
            associated_user: token_account,
            ..Default::default()
        })];
        fill_rpc_event_metadata(&mut events, &meta, &Some(transaction));

        let DexEvent::PumpFunTrade(trade) = &events[0] else {
            panic!("expected PumpFun trade");
        };
        assert_eq!(trade.pre_token_balance, Some(10));
        assert_eq!(trade.post_token_balance, Some(35));
        assert_eq!(trade.pre_sol_balance, Some(50_000));
        assert_eq!(trade.post_sol_balance, Some(40_000));
    }

    #[test]
    fn invalid_rpc_loaded_address_returns_error_instead_of_panicking() {
        let mut rpc_tx = rpc_fixture(Pubkey::new_unique(), Pubkey::new_unique());
        rpc_tx.transaction.meta.as_mut().unwrap().loaded_addresses =
            OptionSerializer::Some(UiLoadedAddresses {
                writable: vec!["not-a-pubkey".to_string()],
                readonly: Vec::new(),
            });

        let error = convert_rpc_to_grpc(&rpc_tx).expect_err("invalid address must fail");
        assert!(
            matches!(error, ParseError::ConversionError(message) if message.contains("Invalid loaded address"))
        );
    }

    #[test]
    fn base58_rpc_transaction_is_supported() {
        let mut rpc_tx = rpc_fixture(Pubkey::new_unique(), Pubkey::new_unique());
        let EncodedTransaction::Binary(data, TransactionBinaryEncoding::Base64) =
            &rpc_tx.transaction.transaction
        else {
            panic!("expected base64 fixture");
        };
        let bytes = general_purpose::STANDARD.decode(data).expect("decode fixture");
        rpc_tx.transaction.transaction = EncodedTransaction::Binary(
            bs58::encode(bytes).into_string(),
            TransactionBinaryEncoding::Base58,
        );

        convert_rpc_to_grpc(&rpc_tx).expect("base58 binary transaction must decode");
    }

    #[test]
    fn unsanitized_rpc_transaction_is_rejected() {
        let mut rpc_tx = rpc_fixture(Pubkey::new_unique(), Pubkey::new_unique());
        let invalid = VersionedTransaction {
            signatures: vec![Signature::from([7; 64])],
            message: VersionedMessage::Legacy(legacy::Message {
                header: MessageHeader { num_required_signatures: 1, ..Default::default() },
                account_keys: vec![Pubkey::new_unique()],
                recent_blockhash: Hash::new_unique(),
                instructions: vec![
                    solana_sdk::message::compiled_instruction::CompiledInstruction {
                        program_id_index: 9,
                        accounts: Vec::new(),
                        data: Vec::new(),
                    },
                ],
            }),
        };
        let bytes = wincode::serialize(&invalid).expect("serialize invalid fixture");
        rpc_tx.transaction.transaction = EncodedTransaction::Binary(
            general_purpose::STANDARD.encode(bytes),
            TransactionBinaryEncoding::Base64,
        );

        let error = convert_rpc_to_grpc(&rpc_tx).expect_err("unsanitized transaction must fail");
        assert!(matches!(
            error,
            ParseError::ConversionError(message) if message.contains("decode or sanitize")
        ));
    }
}
