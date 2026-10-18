# Signed Pump route regression captures

Six successful mainnet transactions reported by a user, fetched on 2026-10-18.
`wire` is the original signed VersionedTransaction in base64. Loaded addresses
are saved RPC execution metadata used as an offline ALT cache. The tests verify
all Ed25519 signatures; no RPC or transaction submission occurs in tests.
Wrapper expected account identities and Rapid input/minimum come independently
from executed inner `buy_exact_quote_in_v2` instructions. BuyPumpDelta inputs
are deliberately unknown to the raw parser because they depend on account state.
Official route identities/bounds follow the signed outer instruction and IDL.
These historical captures are not a bank simulation of the wrapper programs.
