# Documentation

Start with the [example index](../examples/README.md) for runnable commands.

- [Architecture and contribution boundaries](ARCHITECTURE.md): evidence, hot paths and naming.

- [Route analysis](ROUTE_ANALYSIS.md): execution positions, migrations and attribution.
- [Cross-DEX stock routes](cross-dex-stock-routes.md): per-leg identification and signed replays.
- [Stock prices](stock-prices.md): catalog policy, fee evidence and price semantics.
- [Wrapped Pump.fun](wrapped-pumpfun.md): raw provenance, offline analysis and Shred limits.
- [Account subscriptions](ACCOUNT_SUBSCRIPTIONS.md): account data and subscriptions.
- [gRPC versus RPC](grpc-vs-rpc.md): input and execution metadata differences.
- [Latency troubleshooting](troubleshooting-latency.md): transport/performance investigation.
- [CPMM creator fees](cpmm-creator-fee-share.md): protocol-specific fee semantics.
- [Pump account layouts](PUMP_PUMP_AMM_PARSER_ACCOUNTS.md): instruction account reference.

`research/` holds dated evidence, not live prices or current pool guarantees.
[Release history](../CHANGELOG.md) / [中文版本记录](../CHANGELOG_CN.md) and
`releases/` preserve historical notes; consult current topic guides for behavior.
