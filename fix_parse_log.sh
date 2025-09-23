#!/bin/bash

for file in src/logs/{raydium_launchpad,pump_amm,raydium_clmm,raydium_cpmm,raydium_amm,orca_whirlpool,meteora_amm,meteora_damm,meteora_dlmm}.rs; do
  # 找到注释行插入parse_log
  sed -i '' '/^\/\/\/ 主要的.*日志解析函数/a\
pub fn parse_log(log: \&str, signature: Signature, slot: u64, block_time: Option<i64>, _grpc_recv_us: i64) -> Option<DexEvent> {\
    parse_structured_log(log, signature, slot, block_time)\
}\
' "$file"
  echo "Added parse_log to $file"
done