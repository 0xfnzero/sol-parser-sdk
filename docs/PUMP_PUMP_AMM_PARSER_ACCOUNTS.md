# Pump / Pump AMM 解析账号与数据说明

本文档说明 sol-parser-sdk 中 Pump（pumpfun）与 Pump AMM（pumpswap）交易解析所依赖的 IDL、升级后 remaining accounts、账号索引及已覆盖/曾遗漏的账号与数据。IDL 已与 sol-trade-sdk 同步。

## IDL 同步

- **来源**: sol-trade-sdk `idl/`
- **目标**: sol-parser-sdk `idls/`
- **已同步文件**:
  - `idl/pump.json` → `idls/pumpfun.json`（Pump 程序，同 program id）
  - `idl/pump_amm.json` → `idls/pump_amm.json`
  - `idl/pump_fees.json` → `idls/pump_fees.json`（可选，供后续 fee sharing 等解析使用）

## Pump（Bonding Curve）Buy / Sell

### 账号索引（IDL 固定账户 + 升级后 remaining accounts）

**Buy：16 个 IDL 固定账户，升级后交易共 18 个账户**

| 索引 | 账户名 |
|-----|--------|
| 0 | global |
| 1 | fee_recipient |
| 2 | mint |
| 3 | bonding_curve |
| 4 | associated_bonding_curve |
| 5 | associated_user |
| 6 | user |
| 7 | system_program |
| 8 | token_program |
| 9 | creator_vault |
| 10 | event_authority |
| 11 | program |
| 12 | global_volume_accumulator |
| 13 | user_volume_accumulator |
| 14 | fee_config |
| 15 | fee_program |
| 16 | bonding_curve_v2（remaining，升级后实际位置） |
| 17 | buyback_fee_recipient（remaining，8 个新增 fee recipient 之一，mutable） |

说明：旧 IDL/文档容易只看到账户 0–15；升级后的 `bonding_curve_v2` 和 `buyback_fee_recipient` 都在 remaining accounts，分别是 16/17。

**Sell：14 个 IDL 固定账户，升级后非 cashback 交易共 16 个账户，cashback 交易共 17 个账户**

| 索引 | 账户名 |
|-----|--------|
| 0 | global |
| 1 | fee_recipient |
| 2 | mint |
| 3 | bonding_curve |
| 4 | associated_bonding_curve |
| 5 | associated_user |
| 6 | user |
| 7 | system_program |
| 8 | creator_vault |
| 9 | token_program |
| 10 | event_authority |
| 11 | program |
| 12 | fee_config |
| 13 | fee_program |
| 14 | bonding_curve_v2（remaining，非 cashback） / user_volume_accumulator（remaining，cashback） |
| 15 | buyback_fee_recipient（remaining，非 cashback） / bonding_curve_v2（remaining，cashback） |
| 16 | buyback_fee_recipient（remaining，cashback） |

说明：non-cashback sell 的 `bonding_curve_v2` / `buyback_fee_recipient` 分别在 14/15；cashback sell 多一个 `user_volume_accumulator`，因此 `bonding_curve_v2` / `buyback_fee_recipient` 分别后移到 15/16。

### 解析与填充

- **日志解析**（logs/pump.rs, pump_inner.rs）: TradeEvent 含 `creator`、`creator_fee` 等；**creator_vault 不在事件数据中**，在日志解析里置为 `Pubkey::default()`。
- **账户填充**（account_fillers/pumpfun.rs）: 根据指令账户补全 `creator_vault`、`token_program`、`bonding_curve_v2`、`buyback_fee_recipient` 等；Buy 使用索引 9 的 creator_vault，Sell 使用索引 8。**必须通过指令账户填充才能得到正确的 creator_vault**（对 Creator Rewards Sharing 的币，该地址可能为 sharing config PDA）。
- **指令解析**（instr/pump.rs）: Buy/Sell 解析会同时读取固定账户和升级后的 remaining accounts；日志优先路径会通过 merger/filler 补全这些账户字段。

### 曾遗漏 / 注意点

- **creator_vault**：事件数据中无此字段，若不做账户填充会一直为 default。sol-trade-sdk 卖出时需要最新 creator_vault（见 README Creator Rewards Sharing）。确保在 gRPC/RPC 解析链路中调用 `fill_trade_accounts`，以便从指令账户 8/9 填入 creator_vault。
- **bonding_curve_v2、buyback_fee_recipient**：升级后 buy/sell 都需要从 remaining accounts 解析。`buyback_fee_recipient` 是 8 个新增 fee recipient 之一，需 mutable；旧 IDL 不包含这两个字段，不能只按 IDL 固定账户数判断。

## Pump AMM（PumpSwap）Buy / Sell

### 账号索引（IDL 固定账户 + 升级后 remaining accounts）

**Buy：23 个 IDL 固定账户，升级后非 cashback 交易共 26 个账户，cashback 交易共 27 个账户**

| 索引 | 账户名 |
|-----|--------|
| 0 | pool |
| 1 | user |
| 2 | global_config |
| 3 | base_mint |
| 4 | quote_mint |
| 5 | user_base_token_account |
| 6 | user_quote_token_account |
| 7 | pool_base_token_account |
| 8 | pool_quote_token_account |
| 9 | protocol_fee_recipient |
| 10 | protocol_fee_recipient_token_account |
| 11 | base_token_program |
| 12 | quote_token_program |
| 13 | system_program |
| 14 | associated_token_program |
| 15 | event_authority |
| 16 | program |
| 17 | coin_creator_vault_ata |
| 18 | coin_creator_vault_authority |
| 19 | global_volume_accumulator |
| 20 | user_volume_accumulator |
| 21 | fee_config |
| 22 | fee_program |
| 23 | pool_v2（remaining，非 cashback） / cashback extra account（remaining，cashback） |
| 24 | fee_recipient（remaining，非 cashback，只读） / pool_v2（remaining，cashback） |
| 25 | fee_recipient_quote_token_account（remaining，非 cashback，mutable） / fee_recipient（remaining，cashback，只读） |
| 26 | fee_recipient_quote_token_account（remaining，cashback，mutable） |

说明：cashback buy 在 `pool_v2` 前多一个额外账户，因此三项升级账户整体后移一位。解析器只暴露 `pool_v2`、`fee_recipient`、`fee_recipient_quote_token_account`。

**Sell：21 个 IDL 固定账户，升级后非 cashback 交易共 24 个账户，cashback 交易共 26 个账户**

| 索引 | 账户名 |
|-----|--------|
| 0 | pool |
| 1 | user |
| 2 | global_config |
| 3 | base_mint |
| 4 | quote_mint |
| 5 | user_base_token_account |
| 6 | user_quote_token_account |
| 7 | pool_base_token_account |
| 8 | pool_quote_token_account |
| 9 | protocol_fee_recipient |
| 10 | protocol_fee_recipient_token_account |
| 11 | base_token_program |
| 12 | quote_token_program |
| 13 | system_program |
| 14 | associated_token_program |
| 15 | event_authority |
| 16 | program |
| 17 | coin_creator_vault_ata |
| 18 | coin_creator_vault_authority |
| 19 | fee_config |
| 20 | fee_program |
| 21 | pool_v2（remaining，非 cashback） / cashback extra account（remaining，cashback） |
| 22 | fee_recipient（remaining，非 cashback，只读） / cashback extra account（remaining，cashback） |
| 23 | fee_recipient_quote_token_account（remaining，非 cashback，mutable） / pool_v2（remaining，cashback） |
| 24 | fee_recipient（remaining，cashback，只读） |
| 25 | fee_recipient_quote_token_account（remaining，cashback，mutable） |

说明：cashback sell 在 `pool_v2` 前多两个额外账户，因此三项升级账户整体后移两位。

### 解析与填充

- **指令解析**（instr/pump_amm.rs）: 已按 IDL 补全 17、18；当 `accounts.len() >= 19` 时，从 17、18 读取并写入 `coin_creator_vault_ata`、`coin_creator_vault_authority`。升级后的 `pool_v2`、`fee_recipient`、`fee_recipient_quote_token_account` 会根据 26/27 buy、24/26 sell 账户数读取。
- **账户填充**（account_fillers/pumpswap.rs）: 仍从 17、18 填充 `coin_creator_vault_ata`、`coin_creator_vault_authority`（与 IDL 一致），并从升级后的 remaining account 尾部填充 `pool_v2`、`fee_recipient`、`fee_recipient_quote_token_account`。
- **日志解析**（logs/pump_amm.rs）: 事件数据中含 `coin_creator`、`coin_creator_fee` 等；`coin_creator_vault_ata` / `coin_creator_vault_authority` 需由指令账户或填充器提供。

### 曾遗漏 / 已修复

- **coin_creator_vault_ata、coin_creator_vault_authority**：原先指令解析只用到 0–12，未读 17、18。现已在 `parse_buy_instruction`、`parse_buy_exact_quote_in_instruction`、`parse_sell_instruction` 中在 `accounts.len() >= 19` 时写入上述两字段。
- **pool_v2、fee_recipient、fee_recipient_quote_token_account**：升级后 PumpSwap buy/sell 都必须带这三个尾部账户，其中 `fee_recipient` 是 8 个新增 fee recipient 之一，`fee_recipient_quote_token_account` 是该 fee recipient 的 quote mint ATA。

## 数据字段小结

| 程序 | 字段 | 来源 | 说明 |
|------|------|------|------|
| Pump | creator_vault | 指令账户 8(sell)/9(buy)，经 fill_trade_accounts | 必填；Creator Rewards Sharing 时需最新值 |
| Pump | bonding_curve_v2 | remaining account，buy 16；sell 非 cashback 14 / cashback 15 | 升级后必填 |
| Pump | buyback_fee_recipient | remaining account，buy 17；sell 非 cashback 15 / cashback 16 | 8 个新增 fee recipient 之一，mutable |
| Pump | creator, creator_fee 等 | 日志 TradeEvent | 已有 |
| Pump AMM | coin_creator_vault_ata, coin_creator_vault_authority | 指令账户 17、18 | 已补全到指令解析与填充器 |
| Pump AMM | pool_v2 | remaining account，buy 非 cashback 23 / cashback 24；sell 非 cashback 21 / cashback 23 | 升级后必填 |
| Pump AMM | fee_recipient | remaining account，buy 非 cashback 24 / cashback 25；sell 非 cashback 22 / cashback 24 | 8 个新增 fee recipient 之一，只读 |
| Pump AMM | fee_recipient_quote_token_account | remaining account，buy 非 cashback 25 / cashback 26；sell 非 cashback 23 / cashback 25 | fee recipient 的 quote mint ATA，mutable |
| Pump AMM | coin_creator, coin_creator_fee 等 | 日志事件 | 已有 |

## 建议

1. 使用 Pump 事件构建卖出参数时，务必在合并/下发前调用 **fill_trade_accounts**，以便 `creator_vault` 来自当前指令账户，避免 2006 seeds 错误。
2. 不要只依赖 IDL 固定账户数判断升级后交易是否完整；Pump/PumpSwap 的新增账户都在 remaining accounts。
3. 保持 IDL 与 sol-trade-sdk 定期同步（复制 `idl/*.json` → `idls/`），以便新指令或新账户加入时解析与注释仍正确。
