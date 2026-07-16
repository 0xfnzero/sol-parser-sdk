use crate::{core::events::*, instr::read_bool};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

#[inline]
fn set_pumpswap_is_pump_pool_from_fees_ix(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &HashMap<Pubkey, Vec<(i32, i32)>>,
    is_pump_pool: &mut bool,
) {
    if let Some(invoke) =
        program_invokes.get(&crate::grpc::program_ids::PUMPSWAP_FEES_PROGRAM).and_then(|v| v.last())
    {
        if let Some(data) = get_instruction_data(meta, transaction, invoke) {
            *is_pump_pool = read_bool(data, 9).unwrap_or_default();
        }
    }
}

#[inline]
pub fn fill_data(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &HashMap<Pubkey, Vec<(i32, i32)>>,
) {
    match event {
        DexEvent::PumpSwapBuy(ref mut e) => {
            set_pumpswap_is_pump_pool_from_fees_ix(
                meta,
                transaction,
                program_invokes,
                &mut e.is_pump_pool,
            );
        }
        DexEvent::PumpSwapSell(ref mut e) => {
            set_pumpswap_is_pump_pool_from_fees_ix(
                meta,
                transaction,
                program_invokes,
                &mut e.is_pump_pool,
            );
        }
        _ => {}
    }
}

pub fn get_instruction_data<'a>(
    meta: &'a TransactionStatusMeta,
    transaction: &'a Option<Transaction>,
    index: &(i32, i32), // (outer_index, inner_index)
) -> Option<&'a [u8]> {
    let data = if index.1 >= 0 {
        meta.inner_instructions
            .iter()
            .find(|i| i.index == index.0 as u32)?
            .instructions
            .get(index.1 as usize)?
            .data
            .as_slice()
    } else {
        transaction.as_ref()?.message.as_ref()?.instructions.get(index.0 as usize)?.data.as_slice()
    };
    Some(data)
}
