#!/usr/bin/env python3
import re

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

    # Fix: Pubkey::default()), -> Pubkey::default(),
    content = re.sub(r'Pubkey::default\(\)\),', r'Pubkey::default(),', content)

    # Fix: grpc_recv_us)); -> grpc_recv_us);
    content = re.sub(r'grpc_recv_us\)\);', r'grpc_recv_us);', content)

    with open(filepath, 'w') as f:
        f.write(content)

    print(f"Fixed {filepath}")

print("All files fixed!")