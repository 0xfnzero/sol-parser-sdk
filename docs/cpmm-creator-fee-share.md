# CPMM creator-fee protocol share (October 2026)

Source: [Raydium changelog](https://docs.raydium.io/reference/changelog/2026-09-19-cpmm-creator-fee-protocol-share), verified against `raydium-io/raydium-cp-swap` program source.

`RaydiumCpmmAmmConfig.creator_fee_share_rate` decodes the former `padding[0]` slot. Padding is now `[u64; 14]`, so update struct literals and serialized layouts previously using `[u64; 15]`. Config remains 236 bytes including its discriminator; PoolState is unchanged.

`DexEvent::RaydiumCpmmCreatorFeeShareAccount` exposes `bump`, `creator`, `amm_config`, `share_rate` and padding from the 145-byte CreatorFeeShare account. Subscribe/filter with `EventType::AccountRaydiumCpmmCreatorFeeShare` through the normal account parser.

`DexEvent::RaydiumCpmmCollectCreatorFee` parses both creator-signed (15 accounts) and permissionless (16 accounts) collection instructions. Its `permissionless` flag distinguishes them, and its fields include payer, creator, pool, config, vaults, receiving token accounts, token programs and the share PDA. Filter with `EventType::RaydiumCpmmCollectCreatorFee`. These instructions have no amount arguments; the event does not invent executed payout amounts.

Resolve the protocol share from the PDA `[b"creator_fee_share", creator, amm_config]` under the CPMM program when it exists; otherwise use AmmConfig. Rates are read at collection time. Protocol share is `floor(gross_creator_fee * rate / 1_000_000)`; creator receives the remainder, before token transfer fees. Protocol-fee counters now grow on collection as well as swaps; gross creator-fee counters are not net payout estimates.

Both `idl/raydium_cpmm.json` and `idls/raydium_cpmm.json` contain the updated collection account lists, PDA seeds and account layouts. Swap, quote and LP behavior is unchanged.

## Replay example using actual simulation accounts

First generate a report with the trade SDK's `cpmm_creator_fee_simulate` example. From the parser SDK repository, replay it:

```bash
cargo run --example cpmm_creator_fee_replay -- /tmp/cpmm-collection-report.json
```

The example parses each collection instruction, its pre-collection config and custom share account (when present), and the simulated post-collection PoolState. It prints decoded events as JSON lines, using raw RPC-returned account bytes. No network connection is needed for replay.

The trade SDK's captured mainnet regression test compares every decoded config, pool and custom-share field between the two SDKs. The October 4, 2026 fixtures cover the 5% config fallback, a 0% custom PDA override, and nonzero Token-2022 creator payouts with a 1% transfer fee in both collection modes.

The trade SDK's `cpmm_swap_after_collection` regression suite additionally captures eight real swap simulations: exact-input/exact-output swaps, with and without each collection mode. It checks parser-decoded SwapEvent input/output and transfer-fee amounts against trader balance changes, and compares parsed post-swap creator/protocol counters against the trade SDK decoder. This verifies that collection and swap events remain compatible in the same transaction.

The trade SDK's `cpmm_lp_after_collection` suite also validates four mainnet deposit/withdraw comparisons, with and without both collection modes. Parser-decoded post-operation LP supply and creator/protocol counters must match the independent trade SDK decoder. Account deltas and raw `LpChangeEvent` fields verify LP proportional rounding and Token-2022 fees. Captured RPC bytes allow offline replay without network access; LP test instructions are built from the IDL, not a new public trade SDK LP API.

## gRPC/shred data path

For live state, prefer Yellowstone account subscriptions with `EventType::AccountRawSnapshot` and explicit pool/config/CreatorFeeShare PDA address filters. The raw event includes slot, write_version, owner, bytes and zero-lamport closures; feed it to the trade SDK's `SubscriptionAccountCache::update_from_parser_snapshot`. Normalized account events are useful for inspection, but cannot represent an empty/closed account. A stream that has not emitted the PDA does not prove it is absent; obtain a validated snapshot or leave the state unknown.

`ShredStreamClient::subscribe_with_filter` and `parse_transaction_dex_events_with_filter` support `EventType::RaydiumCpmmCollectCreatorFee` for both outer collection instructions without RPC. Shred provides intent, not account state or execution confirmation. Do not use it to clear accrued fees or calculate actual protocol increases. ALT-loaded keys and CPI-only instructions require resolved gRPC transaction data. The regression test covers both collection forms through the actual shred transaction parsing path and verifies event filtering.

The trade SDK's `cpmm_creator_fee_stream` example connects gRPC raw updates to cached payout estimation and instruction preparation, optionally observes shred instructions, and has an offline JSONL replay mode. It performs no RPC bootstrap/fallback, simulation or broadcast. Existing `simulateTransaction` examples remain explicit execution diagnostics.

`YellowstoneGrpc::subscription_status()` exposes client-local connection generation, continuity revision, disconnection count and dropped-event count, shared by client clones. Read it before and after applying a cache batch; a changed revision requires discarding incomplete state and rebuilding it. Successful queue writes do not lock the health state; rare connection/loss transitions and status reads use a consistent snapshot. Queue loss is tracked across raw-account, transaction, buffered and block-metadata event paths. Automatic reconnection does not replay missed account updates or establish fork finality.

Dynamic gRPC account/transaction filter updates now retain BlockMeta subscription and persist the latest accepted filters for automatic reconnect. A full control queue returns a retryable error instead of blocking shutdown, and rejected updates do not replace the stored reconnect configuration. The local Yellowstone transport regression forces a real stream disconnect after a filter update and checks the reconnect request and resumed block progress, without Solana RPC or an external provider.

2026-10-05 审查修复：gRPC 原始快照直接转移输入账户字节，兼容同时订阅标准化事件；账户地址/owner 非 32 字节时丢弃并标记订阅连续性失效。CPMM LP/初始化账户回填按指令种类及池身份唯一匹配，多义时保留未知用户，不从其他 invocation 猜测。

第二轮审查：重连使用 retry_delay_ms（低延迟配置为 100ms），失败连接按指数退避，成功建立流后恢复基础间隔，最大 60 秒；零配置最低 1ms。Unordered 模式不再唤醒无作用的排序计时器。升级辅助函数拒绝将新增 PDA 标为交易签名者，错误时不修改原指令；writable 权限超集仍可接受。

第三轮闭户修复：gRPC 收到 lamports=0 且仍带有旧账户字节的更新时，仅保留显式订阅的 AccountRawSnapshot 闭户事件，不将旧字节解码为有效标准化状态。需要处理账户删除的订阅者应消费原始快照并应用闭户逻辑。
