# Troubleshooting latency measurements

A negative duration usually indicates that timestamps came from different clocks, machines, or parsing entry points. `block_time_us` is a chain timestamp; it is not the local gRPC receipt timestamp and cannot measure local parser latency.

## Use one clock for both endpoints

For an event whose receipt timestamp uses the SDK high-performance clock, calculate the duration with that same clock:

```rust
use sol_parser_sdk::core::now_micros;

let elapsed_us = now_micros() - event.metadata().grpc_recv_us;
```

When supplying `grpc_recv_us` to a parser directly, use the same clock that will measure the end of the operation. For code you own, `std::time::Instant` is suitable for measuring a local elapsed duration.

## Check the parsing path

The legacy `parse_log_unified` convenience entry creates a timestamp via `core::clock::now_us()`. On Unix this uses a system realtime clock, while `now_micros()` uses a process-local monotonic clock anchored to a UTC sample. Do not assume these timestamps stay synchronized after NTP or system clock adjustments. The program-aware log entry accepts an explicit receipt timestamp.

Do not subtract timestamps from different processes or hosts without a known clock relationship. Replayed historical transactions and synthetic fixtures also require explicit timestamp handling. Check timestamp units: SDK metadata uses microseconds.

## Interpret results

Measure parsing, queueing and end-to-end network delay separately. Ordering and batching modes can add intentional waiting. A positive duration does not prove a fresh account snapshot or a successful transaction, and no fixed latency range is guaranteed across hardware and network conditions.

See [clock implementation](../src/core/clock.rs), [log parsing entries](../src/logs/mod.rs), and [gRPC versus RPC](grpc-vs-rpc.md).
