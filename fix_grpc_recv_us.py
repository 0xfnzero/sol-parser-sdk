#!/usr/bin/env python3
import re
import sys

files = [
    'src/logs/meteora_dlmm.rs',
    'src/logs/meteora_damm.rs',
    'src/logs/meteora_amm.rs',
    'src/logs/orca_whirlpool.rs',
    'src/logs/raydium_launchpad.rs',
    'src/logs/pump_amm.rs',
    'src/logs/raydium_clmm.rs',
    'src/logs/raydium_cpmm.rs',
    'src/logs/raydium_amm.rs',
]

for filepath in files:
    print(f"Fixing {filepath}...")

    with open(filepath, 'r') as f:
        content = f.read()

    # 1. Fix parse_log wrapper: _grpc_recv_us -> grpc_recv_us
    content = re.sub(
        r'pub fn parse_log\(log: &str, signature: Signature, slot: u64, block_time: Option<i64>, _grpc_recv_us: i64\)',
        r'pub fn parse_log(log: &str, signature: Signature, slot: u64, block_time: Option<i64>, grpc_recv_us: i64)',
        content
    )

    # 2. Fix parse_log call to parse_structured_log
    content = re.sub(
        r'parse_structured_log\(log, signature, slot, block_time\)',
        r'parse_structured_log(log, signature, slot, block_time, grpc_recv_us)',
        content
    )

    # 3. Fix parse_structured_log signature
    content = re.sub(
        r'(fn parse_structured_log\(\s*log: &str,\s*signature: Signature,\s*slot: u64,\s*block_time: Option<i64>,)\s*\)',
        r'\1\n    grpc_recv_us: i64,\n)',
        content,
        flags=re.MULTILINE
    )

    # 4. Fix all create_metadata_simple calls
    content = re.sub(
        r'create_metadata_simple\(signature, slot, block_time, ([^)]+)\)',
        r'create_metadata_simple(signature, slot, block_time, \1, grpc_recv_us)',
        content
    )

    # 5. Fix all parse_*_event() calls in match arms
    content = re.sub(
        r'parse_(\w+)_event\(data, signature, slot, block_time\)',
        r'parse_\1_event(data, signature, slot, block_time, grpc_recv_us)',
        content
    )

    # 6. Fix all parse_*_event() function signatures
    content = re.sub(
        r'(fn parse_\w+_event\(\s*data: &\[u8\],\s*signature: Signature,\s*slot: u64,\s*block_time: Option<i64>,)\s*\)',
        r'\1\n    grpc_recv_us: i64,\n)',
        content,
        flags=re.MULTILINE
    )

    # 7. Fix parse_text_log signature
    content = re.sub(
        r'(fn parse_text_log\(\s*log: &str,\s*signature: Signature,\s*slot: u64,\s*block_time: Option<i64>,)\s*\)',
        r'\1\n    grpc_recv_us: i64,\n)',
        content,
        flags=re.MULTILINE
    )

    # 8. Fix all parse_*_from_text() calls
    content = re.sub(
        r'parse_(\w+)_from_text\(log, signature, slot, block_time\)',
        r'parse_\1_from_text(log, signature, slot, block_time, grpc_recv_us)',
        content
    )

    # 9. Fix all parse_*_from_text() signatures
    content = re.sub(
        r'(fn parse_\w+_from_text\(\s*log: &str,\s*signature: Signature,\s*slot: u64,\s*block_time: Option<i64>,)\s*\)',
        r'\1\n    grpc_recv_us: i64,\n)',
        content,
        flags=re.MULTILINE
    )

    with open(filepath, 'w') as f:
        f.write(content)

    print(f"Fixed {filepath}")

print("All files fixed!")