# sol-parser-sdk v0.8.0

Uniform trade provenance, opt-in stock prices, and native-language examples.

## Changes

- Supported gRPC trade families carry optional original router instructions and TradeParseStatus. Unknown Pump.fun wrappers on Shred produce unresolved candidates without inventing direction or execution amounts.
- Opt-in stock prices share the screened multi-issuer catalog with trade SDKs. Catalog admission uses $500 liquid-side reserve and no volume threshold; known lower fees are preferred. Historical catalog data is not live liquidity.
- Missing conversion/liquidity observations stay unknown. Snapshot validation rejects blank sources and invalid price-impact bounds.
- Organize examples and documentation; tests use the SDK's own language. Offline captured transactions do not represent fresh mainnet execution or production latency measurements.

## Migration

third_program now contains RawInstructionPayload rather than a rule identifier. It does not contain duplicated metadata/dexes; metadata remains on the event. Check parse_status: Unresolved is only a possible DEX interaction, InstructionDecoded establishes input decoding, and ExecutionConfirmed reflects supplied execution evidence, not necessarily known fill amounts. Do not treat zero defaults as proven zero fills. Rust callers constructing trade structs must initialize the new fields or use ..Default::default().

No RPC is performed by the parsing hot path. Explicit analysis/discovery tools can fetch RPC data outside that path. Wrapper reconstruction remains opt-in and is not proof of future third-party program behavior.
