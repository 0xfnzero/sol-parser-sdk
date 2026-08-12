# IDL Sources

Current IDLs are synced from protocol-maintained repositories or packages. JSON key order and
formatting may differ; use a canonical JSON hash (`jq -S -c .`) when comparing copies.

| Local file | Upstream source |
| --- | --- |
| `pumpfun.json` | `pump-fun/pump-public-docs/idl/pump.json` |
| `pump_amm.json` | `pump-fun/pump-public-docs/idl/pump_amm.json` |
| `pump_fees.json` | `pump-fun/pump-public-docs/idl/pump_fees.json` |
| `raydium_clmm.json` | `raydium-io/raydium-idl/raydium_clmm/raydium_clmm.json` |
| `raydium_cpmm.json` | `raydium-io/raydium-idl/raydium_cpmm/raydium_cp_swap.json` |
| `raydium_launchpad.json` | `raydium-io/raydium-idl/raydium_launchpad/raydium_launchpad.json` |
| `meteora_amm.json` | `MeteoraAg/dynamic-bonding-curve/idls/dynamic_amm.json` |
| `meteora_damm_v2.json` | `MeteoraAg/damm-v2-sdk/src/idl/cp_amm.json` |
| `meteora_dynamic_bonding_curve.json` | `MeteoraAg/dynamic-bonding-curve-sdk/packages/dynamic-bonding-curve/src/idl/dynamic-bonding-curve/idl.json` |
| `meteora_dlmm.json` | `MeteoraAg/dlmm-sdk/idls/dlmm.json` |
| `orca_whirlpool.json` | `@orca-so/whirlpools-sdk/dist/artifacts/whirlpool.json` |

Raydium AMM V4 is not an Anchor program and has no current protocol-maintained Anchor IDL. Its
instruction and `ray_log` layouts are validated against `raydium-io/raydium-amm` source. The
`raydium_amm_v4.json` and `raydium_pool_v4.json` files remain compatibility references.
