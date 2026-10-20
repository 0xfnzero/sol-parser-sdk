# Parser boundaries and contribution guide

Transport adapters in `grpc/`, `rpc_parser.rs` and `shredstream/` supply
different evidence. Instruction decoders in `instr/` decode inputs; log decoders
in `logs/` decode execution events. `core/` reconciles identities, fills context
and exposes event types. Preserve these distinctions: successful invocation does
not imply that every executed amount is known, and Shred cannot reconstruct CPI
execution from an unknown wrapper's accounts alone.

Keep protocol Pubkeys in `instr/program_ids.rs`. Existing gRPC names remain
compatibility exports; do not duplicate their byte values in a new registry.
Use official IDL names for instructions and keep established public event names.
In particular, StonkFun attribution and the DEX executing a route leg are distinct.

The hot path performs no RPC. Resolve ALT data through supplied state, retain
invocation identity through merging, and materialize owned wrapper payloads
after reconciliation. Do not merge incomplete repeated invocation lanes by
guessing their ordinal positions. Large event payloads remain explicit types;
avoid hiding field semantics behind broad macros merely to reduce line count.
Creation context is resolved by mint from outer and inner instructions after
merging. Avoid a second outer-only pre-scan. Same-transaction creation enrichment
skips ordinary swaps without create events before building any lookup maps.

`StockPriceTracker` consumes confirmed execution evidence and maintains bounded,
configured-pool state. Decimal scales and catalog indexes are built at setup;
price callbacks do not reload JSON or fetch liquidity. Reported trade prices,
historical fee snapshots and executable quotes are separate concepts.

Examples share configuration, capture loading and gRPC replay under
`examples/support/`. Keep unrelated protocol diagnostics separate. Use the
[example index](../examples/README.md) for commands, and topic guides for behavior.
The two independently published SDKs package their own catalog; generate/check
those copies with the Rust `stock_catalog_sync` example rather than depending on a
sibling checkout at runtime.

Validate changes with captured execution fixtures, malformed-input cases and
RPC/sequential-gRPC/parallel-gRPC agreement. Measure latency with release builds,
fixed input and documented hardware/load; allocation reductions alone do not
establish a production p99 improvement. See [latency diagnostics](troubleshooting-latency.md).
