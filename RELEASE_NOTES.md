# sol-parser-sdk v0.10.0

Released 2026-10-26.

Fix ShredStream create_v2 callbacks with partially resolved ALT accounts and official PumpSwap multi_hop_swap callbacks through Pump.fun curves. Preserve instruction names, input/limit semantics, unknown quotes and create-before-trade ordering; never infer SOL paid from a non-SOL quote. Synchronize native replay examples and validation across languages.

修复 ALT 账户未完全解析时 create_v2 回调和官方多跳内盘回调，保留正确指令名、输入/下限语义、未知报价及创建先于交易的顺序；同步各语言回放示例。

## Validation / 验证

Native-language regression suites and replay/build checks passed for the synchronized changes. Four trade implementations completed curve buy/sell and pool buy/sell unsigned mainnet simulations (16 successful simulations); these small-amount interface checks do not replace fresh production quotes. No transaction was broadcast.
