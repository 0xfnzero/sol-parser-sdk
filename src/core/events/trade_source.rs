//! Uniform provenance access for supported trade/swap event families.
use super::{DexEvent, RawInstructionPayload};
use solana_sdk::pubkey::Pubkey;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::common_filler::fill_trade_execution_source;
    use crate::instr::program_ids::*;
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, Message, Transaction, TransactionStatusMeta,
    };

    #[test]
    fn every_trade_family_preserves_exact_outer_payload_and_direct_calls_stay_native() {
        let wrapper = Pubkey::new_unique();
        let loaded = Pubkey::new_unique();
        for (mut event, native) in [
            (DexEvent::PumpFunTrade(Default::default()), PUMPFUN_PROGRAM_ID),
            (DexEvent::PumpFunBuy(Default::default()), PUMPFUN_PROGRAM_ID),
            (DexEvent::PumpFunSell(Default::default()), PUMPFUN_PROGRAM_ID),
            (DexEvent::PumpFunBuyExactSolIn(Default::default()), PUMPFUN_PROGRAM_ID),
            (DexEvent::PumpSwapTrade(Default::default()), PUMPSWAP_PROGRAM_ID),
            (DexEvent::PumpSwapBuy(Default::default()), PUMPSWAP_PROGRAM_ID),
            (DexEvent::PumpSwapSell(Default::default()), PUMPSWAP_PROGRAM_ID),
            (DexEvent::RaydiumLaunchlabTrade(Default::default()), RAYDIUM_LAUNCHLAB_PROGRAM_ID),
            (DexEvent::RaydiumCpmmSwap(Default::default()), RAYDIUM_CPMM_PROGRAM_ID),
            (DexEvent::RaydiumClmmSwap(Default::default()), RAYDIUM_CLMM_PROGRAM_ID),
            (DexEvent::RaydiumAmmV4Swap(Default::default()), RAYDIUM_AMM_V4_PROGRAM_ID),
            (DexEvent::OrcaWhirlpoolSwap(Default::default()), ORCA_WHIRLPOOL_PROGRAM_ID),
            (DexEvent::MeteoraPoolsSwap(Default::default()), METEORA_POOLS_PROGRAM_ID),
            (DexEvent::MeteoraDammV2Swap(Default::default()), METEORA_DAMM_V2_PROGRAM_ID),
            (DexEvent::MeteoraDlmmSwap(Default::default()), METEORA_DLMM_PROGRAM_ID),
            (DexEvent::MeteoraDbcSwap(Default::default()), METEORA_DBC_PROGRAM_ID),
        ] {
            let transaction = Some(Transaction {
                message: Some(Message {
                    account_keys: vec![wrapper.to_bytes().to_vec(), native.to_bytes().to_vec()],
                    instructions: vec![
                        CompiledInstruction {
                            program_id_index: 0,
                            accounts: vec![1, 2, 2, 250],
                            data: vec![7, 9],
                        },
                        CompiledInstruction {
                            program_id_index: 1,
                            accounts: vec![],
                            data: vec![8],
                        },
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            });
            let meta = TransactionStatusMeta {
                loaded_writable_addresses: vec![loaded.to_bytes().to_vec()],
                ..Default::default()
            };
            let mut direct = event.clone();
            fill_trade_execution_source(&mut direct, 1, &meta, &transaction);
            assert!(direct.third_program().is_none());
            fill_trade_execution_source(&mut event, 0, &meta, &transaction);
            let raw = event.third_program().unwrap();
            assert_eq!(raw.program_id, wrapper);
            assert_eq!(raw.instruction_index, 0);
            assert_eq!(raw.account_indices, vec![1, 2, 2, 250]);
            assert_eq!(raw.accounts, vec![Some(native), Some(loaded), Some(loaded), None]);
            assert_eq!(raw.instruction_data, vec![7, 9]);
            let json = serde_json::to_value(&event).unwrap();
            let restored: DexEvent = serde_json::from_value(json).unwrap();
            assert_eq!(restored.third_program(), Some(raw));
        }
    }
}

impl DexEvent {
    /// Original outer router instruction, when exact invocation provenance exists.
    pub fn third_program(&self) -> Option<&RawInstructionPayload> {
        match self {
            Self::PumpFunTrade(t)
            | Self::PumpFunBuy(t)
            | Self::PumpFunSell(t)
            | Self::PumpFunBuyExactSolIn(t) => t.third_program.as_ref(),
            Self::PumpSwapTrade(t) => t.third_program.as_ref(),
            Self::PumpSwapBuy(t) => t.third_program.as_ref(),
            Self::PumpSwapSell(t) => t.third_program.as_ref(),
            Self::RaydiumLaunchlabTrade(t) => t.third_program.as_ref(),
            Self::RaydiumCpmmSwap(t) => t.third_program.as_ref(),
            Self::RaydiumClmmSwap(t) => t.third_program.as_ref(),
            Self::RaydiumAmmV4Swap(t) => t.third_program.as_ref(),
            Self::OrcaWhirlpoolSwap(t) => t.third_program.as_ref(),
            Self::MeteoraPoolsSwap(t) => t.third_program.as_ref(),
            Self::MeteoraDammV2Swap(t) => t.third_program.as_ref(),
            Self::MeteoraDlmmSwap(t) => t.third_program.as_ref(),
            Self::MeteoraDbcSwap(t) => t.third_program.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn trade_source_mut(
        &mut self,
    ) -> Option<(&mut Option<RawInstructionPayload>, Pubkey)> {
        use crate::instr::program_ids::*;
        match self {
            Self::PumpFunTrade(t)
            | Self::PumpFunBuy(t)
            | Self::PumpFunSell(t)
            | Self::PumpFunBuyExactSolIn(t) => Some((&mut t.third_program, PUMPFUN_PROGRAM_ID)),
            Self::PumpSwapTrade(t) => Some((&mut t.third_program, PUMPSWAP_PROGRAM_ID)),
            Self::PumpSwapBuy(t) => Some((&mut t.third_program, PUMPSWAP_PROGRAM_ID)),
            Self::PumpSwapSell(t) => Some((&mut t.third_program, PUMPSWAP_PROGRAM_ID)),
            Self::RaydiumLaunchlabTrade(t) => {
                Some((&mut t.third_program, RAYDIUM_LAUNCHLAB_PROGRAM_ID))
            }
            Self::RaydiumCpmmSwap(t) => Some((&mut t.third_program, RAYDIUM_CPMM_PROGRAM_ID)),
            Self::RaydiumClmmSwap(t) => Some((&mut t.third_program, RAYDIUM_CLMM_PROGRAM_ID)),
            Self::RaydiumAmmV4Swap(t) => Some((&mut t.third_program, RAYDIUM_AMM_V4_PROGRAM_ID)),
            Self::OrcaWhirlpoolSwap(t) => Some((&mut t.third_program, ORCA_WHIRLPOOL_PROGRAM_ID)),
            Self::MeteoraPoolsSwap(t) => Some((&mut t.third_program, METEORA_POOLS_PROGRAM_ID)),
            Self::MeteoraDammV2Swap(t) => Some((&mut t.third_program, METEORA_DAMM_V2_PROGRAM_ID)),
            Self::MeteoraDlmmSwap(t) => Some((&mut t.third_program, METEORA_DLMM_PROGRAM_ID)),
            Self::MeteoraDbcSwap(t) => Some((&mut t.third_program, METEORA_DBC_PROGRAM_ID)),
            _ => None,
        }
    }
}
