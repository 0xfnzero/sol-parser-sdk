#!/bin/bash

# Fix all protocol parser files to add grpc_recv_us parameter

for file in src/logs/{meteora_dlmm,meteora_damm,meteora_amm,orca_whirlpool,raydium_launchpad,pump_amm,raydium_clmm,raydium_cpmm,raydium_amm}.rs; do
  echo "Fixing $file..."

  # Fix parse_log wrapper
  sed -i '' 's/pub fn parse_log(log: \&str, signature: Signature, slot: u64, block_time: Option<i64>, _grpc_recv_us: i64)/pub fn parse_log(log: \&str, signature: Signature, slot: u64, block_time: Option<i64>, grpc_recv_us: i64)/g' "$file"
  sed -i '' 's/parse_structured_log(log, signature, slot, block_time)$/parse_structured_log(log, signature, slot, block_time, grpc_recv_us)/g' "$file"

  # Fix parse_structured_log signature
  sed -i '' 's/fn parse_structured_log(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n)/fn parse_structured_log(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n    grpc_recv_us: i64,\n)/g' "$file"

  # Fix all create_metadata_simple calls (4 params -> 5 params)
  sed -i '' 's/create_metadata_simple(signature, slot, block_time, \([^)]*\))$/create_metadata_simple(signature, slot, block_time, \1, grpc_recv_us)/g' "$file"

  # Fix all parse event function calls to add grpc_recv_us
  sed -i '' 's/parse_\([a-z_]*\)_event(data, signature, slot, block_time)$/parse_\1_event(data, signature, slot, block_time, grpc_recv_us)/g' "$file"

  # Fix all parse event function signatures
  sed -i '' 's/fn parse_\([a-z_]*\)_event(\n    data: \&\[u8\],\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n)/fn parse_\1_event(\n    data: \&[u8],\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n    grpc_recv_us: i64,\n)/g' "$file"

  # Fix parse_text_log signature
  sed -i '' 's/fn parse_text_log(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n)/fn parse_text_log(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n    grpc_recv_us: i64,\n)/g' "$file"

  # Fix parse_*_from_text calls
  sed -i '' 's/parse_\([a-z_]*\)_from_text(log, signature, slot, block_time)$/parse_\1_from_text(log, signature, slot, block_time, grpc_recv_us)/g' "$file"

  # Fix parse_*_from_text signatures
  sed -i '' 's/fn parse_\([a-z_]*\)_from_text(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n)/fn parse_\1_from_text(\n    log: \&str,\n    signature: Signature,\n    slot: u64,\n    block_time: Option<i64>,\n    grpc_recv_us: i64,\n)/g' "$file"

  echo "Fixed $file"
done

echo "All files fixed!"