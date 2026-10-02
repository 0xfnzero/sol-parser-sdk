#!/usr/bin/env python3
"""Capture a reproducible, bounded StonkFun corpus without submitting transactions.

RPC_URL is optional. Raw RPC results and the sampling manifest are saved outside
the repository by default. Do not commit credentials or the entire raw corpus.
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import time
import urllib.request

TARGETS = {
    "standard": "4E876qZTE9FJMrBzgVtBrSrzz2TLivB5Y5QXPjB4gZL7",
    "reward": "6BwHHDg3u1854jC8PDLXvR4spTcLNaoBxLJNGC4nTESt",
    "graduated_knots": "BUVzsLLLG7GWoyJVoU31pXiBveazA6GXTavZ9VD3CwS9",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--output", type=Path, default=Path("/tmp/stonkfun-corpus"))
    args = parser.parse_args()
    if not 1 <= args.limit <= 1000:
        parser.error("limit must be between 1 and 1000 per target")
    url = os.environ.get("RPC_URL", "https://api.mainnet-beta.solana.com")
    args.output.mkdir(parents=True, exist_ok=True)

    def rpc(method, params):
        for attempt in range(4):
            request = urllib.request.Request(
                url,
                data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method,
                                 "params": params}).encode(),
                headers={"Content-Type": "application/json"},
            )
            try:
                with urllib.request.urlopen(request, timeout=25) as response:
                    result = json.load(response)
                if "error" in result:
                    raise RuntimeError(str(result["error"]))
                return result["result"]
            except Exception:
                if attempt == 3:
                    raise
                time.sleep(1 + attempt * 2)

    manifest = {"captured_at_unix": int(time.time()), "limit_per_target": args.limit,
                "targets": {}, "transactions": {}}
    for name, address in TARGETS.items():
        rows = rpc("getSignaturesForAddress", [address, {"limit": args.limit}])
        manifest["targets"][name] = {"address": address, "signatures": rows}
    signatures = list(dict.fromkeys(row["signature"]
                                   for target in manifest["targets"].values()
                                   for row in target["signatures"]))

    def fetch(signature):
        try:
            for encoding in ("json", "base64"):
                path = args.output / f"{signature}.{encoding}.json"
                if path.exists():
                    continue
                result = rpc("getTransaction", [signature, {
                    "encoding": encoding, "maxSupportedTransactionVersion": 1,
                }])
                if result is None:
                    raise RuntimeError("Transaction unavailable from RPC")
                path.write_text(json.dumps(result))
            return signature, {"captured": True}
        except Exception as error:
            # URL / credentials are intentionally excluded from diagnostics.
            return signature, {"captured": False, "error_type": type(error).__name__}

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        for signature, result in pool.map(fetch, signatures):
            manifest["transactions"][signature] = result
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2))
    print(json.dumps({"output": str(args.output), "unique_signatures": len(signatures),
                      "captured": sum(r["captured"] for r in manifest["transactions"].values())}))


if __name__ == "__main__":
    main()
