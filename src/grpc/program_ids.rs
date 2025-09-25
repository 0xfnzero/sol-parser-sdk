// Program IDs for supported DEX protocols
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMPSWAP_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const BONK_PROGRAM_ID: &str = "BSwp6bEBihVLdqJRKS58NaebUBSDNjN7MdpFwNaR6gn3";
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQtcaMpgYqJPXBDvfE";
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

use crate::grpc::types::Protocol;
use std::collections::HashMap;

lazy_static::lazy_static! {
    pub static ref PROTOCOL_PROGRAM_IDS: HashMap<Protocol, Vec<&'static str>> = {
        let mut map = HashMap::new();
        map.insert(Protocol::PumpFun, vec![PUMPFUN_PROGRAM_ID]);
        map.insert(Protocol::PumpSwap, vec![PUMPSWAP_PROGRAM_ID]);
        map.insert(Protocol::Bonk, vec![BONK_PROGRAM_ID]);
        map.insert(Protocol::RaydiumCpmm, vec![RAYDIUM_CPMM_PROGRAM_ID]);
        map.insert(Protocol::RaydiumClmm, vec![RAYDIUM_CLMM_PROGRAM_ID]);
        map.insert(Protocol::RaydiumAmmV4, vec![RAYDIUM_AMM_V4_PROGRAM_ID]);
        // 移除不存在的协议，只保留有实际常量的协议
        map
    };
}

pub fn get_program_ids_for_protocols(protocols: &[Protocol]) -> Vec<String> {
    let mut program_ids = Vec::new();
    for protocol in protocols {
        if let Some(ids) = PROTOCOL_PROGRAM_IDS.get(protocol) {
            for id in ids {
                program_ids.push(id.to_string());
            }
        }
    }
    program_ids.sort();
    program_ids.dedup();
    program_ids
}