# gRPC vs RPC Parsing: Understanding the Differences

## Overview

This document explains the fundamental differences between gRPC and RPC parsing methods in sol-parser-sdk, and why a transaction might be parseable via RPC but not via gRPC.

## Core Differences

### gRPC (Yellowstone)

**Type**: Real-time Subscription System

**Characteristics**:
- ✅ Ultra-low latency (10-20μs)
- ✅ Real-time transaction streaming
- ✅ Ideal for MEV, arbitrage, and live monitoring
- ❌ **Cannot access historical transactions**
- ❌ Only receives transactions **after** subscription starts

**Use Cases**:
- Real-time DEX event monitoring
- MEV bots and arbitrage
- Live trading systems
- High-frequency trading

### RPC

**Type**: Historical Query System

**Characteristics**:
- ✅ Can query any historical transaction by signature
- ✅ Access to complete on-chain data
- ✅ Perfect for testing and validation
- ❌ Higher latency (hundreds of milliseconds)
- ❌ Rate limited by RPC endpoints

**Use Cases**:
- Historical data analysis
- Transaction validation
- Testing and debugging
- Data backfilling

## Why Can't gRPC Parse Historical Transactions?

### Example Case

Transaction signature: `5curEt85cQhAK6R9pntSJ4fmYCiPEG22NjZyGrnGSbNwAkHJMN25T9Efp1n9Tf9vGXhnDXMQYrCNpoRHQTMcZ1s9`

- **Time**: 2026-01-12 23:34:16
- **Slot**: 393033163
- **Status**: Historical transaction (occurred yesterday)

### Why gRPC Can't Parse It

1. **gRPC is a streaming subscription**
   - Only receives transactions that occur **after** the subscription is established
   - Cannot retroactively fetch historical data
   - Designed for real-time use cases, not historical queries

2. **RPC can parse it**
   - RPC allows querying any transaction by its signature
   - Can access the complete historical ledger
   - This is why `parse_transaction_from_rpc` works

## Parsing Logic Consistency

**Important**: Both RPC and gRPC parsers use **identical core parsing logic**:

```rust
// Shared parsing functions used by both RPC and gRPC:

1. crate::logs::parse_log                        // Log parsing
2. fill_accounts_from_transaction_data           // Account filling
3. common_filler::fill_data                      // Data filling
```

RPC replay helps validate gRPC parsing when both inputs contain equivalent
execution metadata. Missing logs, CPI records or loaded addresses can change
results; a successful RPC replay does not verify live subscription delivery.

## How to Verify Parsing Works

### Method 1: Verify Historical Transaction with RPC (Recommended)

```bash
cargo run --example parse_pump_tx --release
```

This will:
- Fetch the transaction from RPC
- Parse using the same logic as gRPC
- Confirm that parsing logic is correct

### Method 2: Monitor Real-Time Transactions with gRPC

```bash
cargo run --example pumpfun_quick_test --release
```

This will:
- Subscribe to live PumpFun events
- Receive and parse real-time transactions
- Verify gRPC streaming works correctly

### Method 3: Preserve Transaction Details and Replay Offline

```bash
cargo run --example rpc_transaction_replay -- --fetch <SIGNATURE> /tmp/transaction.json
cargo run --example rpc_transaction_replay -- --fixture /tmp/transaction.json
```

The saved JSON retains the original transaction, metadata, instructions and logs.
Inspect that file when debugging account mapping; replaying it exercises the
current public parser without another RPC request. Choose a new output filename
when preserving multiple captures.

## Common Questions

### Q: "Why can't gRPC see my transaction?"

**A**: If your transaction is historical (already confirmed on-chain), gRPC cannot access it. gRPC only streams new transactions that occur after subscription starts.

**Solution**: Use RPC to query historical transactions.

### Q: "How do I test if my gRPC parsing works?"

**A**: Two approaches:
1. Use RPC to test the parsing logic (same code, can access history)
2. Run gRPC subscription and wait for new transactions

### Q: "Are RPC and gRPC parsing results identical?"

**A**: Yes! They use the exact same parsing functions:
- Same log parsing (`parse_log`)
- Same account filling (`fill_accounts_from_transaction_data`)
- Same data filling (`fill_data`)

The only difference is the data source (RPC query vs gRPC stream).

## Conclusion

If users report "gRPC can't parse this transaction", check if it's a historical transaction:

1. ✅ **Historical transaction** → This is expected behavior
   - Solution: Use RPC to verify parsing works
   - RPC and gRPC use same logic, so RPC success = gRPC will work for live txs

2. ❌ **Real-time transaction** → This indicates a parsing issue
   - Investigate the parsing logic
   - Check subscription filters
   - Verify gRPC connection

## Further Reading

- [Examples Documentation](../examples/)
- [PumpFun Parsing Examples](../examples/pumpfun_quick_test.rs)
- [RPC Parser Implementation](../src/rpc_parser.rs)
- [gRPC Client Implementation](../src/grpc/client.rs)
