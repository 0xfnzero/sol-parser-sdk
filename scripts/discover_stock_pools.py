#!/usr/bin/env python3
"""Bounded read-only mainnet discovery. Never submit transactions or install pools.
Uses exact official mint identities and records every RPC gap. The two Stonk
addresses are platform accounts, not program IDs. HTTP(S)_PROXY is supported.
"""
import argparse
import concurrent.futures
import datetime
import json
from pathlib import Path
import time
import urllib.request

TARGETS = {
    'stonk': '4E876qZTE9FJMrBzgVtBrSrzz2TLivB5Y5QXPjB4gZL7',
    'stonk_reward': '6BwHHDg3u1854jC8PDLXvR4spTcLNaoBxLJNGC4nTESt',
    'pumpfun': '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P',
    'pumpswap': 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA',
    'raydium_amm': '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8',
    'raydium_cpmm': 'CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C',
    'raydium_clmm': 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
}
POOL_INDICES = {
    'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo': 0,
    'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK': 2,
    'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc': 4,
    'CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C': 3,
    '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8': 1,
}


def rpc(endpoint, method, params):
    for attempt in range(3):
        try:
            body = json.dumps({'jsonrpc':'2.0','id':1,'method':method,'params':params}).encode()
            request = urllib.request.Request(endpoint, data=body, headers={'Content-Type':'application/json', 'User-Agent':'sol-stock-discovery/1.0'})
            with urllib.request.urlopen(request, timeout=20) as response:
                result = json.load(response)
            if 'error' in result:
                raise ValueError(str(result['error']))
            if result.get('result') is None:
                raise ValueError('RPC result unavailable')
            return result['result']
        except Exception:
            if attempt == 2:
                raise
            time.sleep(attempt + 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint', default='https://solana-rpc.publicnode.com')
    parser.add_argument('--limit', type=int, default=1000)
    parser.add_argument('--identities', type=Path)
    parser.add_argument('--xstocks', type=Path)
    parser.add_argument('--backpack', type=Path)
    parser.add_argument('--replay', action='store_true', help='Reuse the saved signature window without listing new signatures')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not 1 <= args.limit <= 1000:
        parser.error('limit must be between 1 and 1000 per source')
    official = json.loads(args.identities.read_text())['stock_mints'] if args.identities else {}
    if args.backpack:
        for record in json.loads(args.backpack.read_text()):
            if record['symbol'].endswith('.US'):
                for token in record['tokens']:
                    if token['blockchain'] == 'Solana' and token.get('contractAddress'):
                        official[token['contractAddress']] = record
    if args.xstocks:
        for record in json.loads(args.xstocks.read_text())['props']['pageProps']['products']:
            mint = record.get('addresses', {}).get('solana')
            if mint:
                official[mint] = {'symbol':record['symbol'], 'issuer_source':'https://xstocks.com/products', 'issuer_record':record}
    if not official:
        parser.error('provide official identities or xstocks/backpack directories')
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = {'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'limit_per_source':args.limit, 'sources':{}, 'fetch_failures':{}, 'pools':{}, 'matched_mints':{}}
    saved = json.loads((args.output/'manifest.json').read_text()) if args.replay else None
    manifest['source_failures'] = {}
    signatures = set()
    for name, address in TARGETS.items():
        try:
            rows = saved['sources'][name]['rows'] if saved else rpc(args.endpoint, 'getSignaturesForAddress', [address, {'limit':args.limit,'commitment':'confirmed'}])
        except Exception as error:
            manifest['source_failures'][name] = str(error)
            continue
        manifest['sources'][name] = {'address':address,'rows':rows}
        print(f'{name}: {len(rows)} signatures', flush=True)
        signatures.update(row['signature'] for row in rows if row['err'] is None)
    (args.output/'manifest.json').write_text(json.dumps(manifest, indent=2))
    def fetch(signature):
        path = args.output / (signature + '.json')
        try:
            tx = json.loads(path.read_text()) if path.exists() else rpc(args.endpoint,
                'getTransaction', [signature, {'encoding':'json','maxSupportedTransactionVersion':1}])
            if not path.exists():
                path.write_text(json.dumps(tx))
            return signature, tx, None
        except Exception as error:
            return signature, None, str(error)
    count = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        for signature, tx, error in executor.map(fetch, sorted(signatures)):
            if error:
                manifest['fetch_failures'][signature] = error
                continue
            count += 1
            if tx['meta']['err'] is not None:
                continue
            mints = {b['mint'] for b in tx['meta'].get('postTokenBalances', []) if b['mint'] in official}
            if mints:
                for mint in mints:
                    manifest['matched_mints'].setdefault(mint, []).append(signature)
                message = tx['transaction']['message']
                keys = message['accountKeys'] + tx['meta'].get('loadedAddresses',{}).get('writable',[]) + tx['meta'].get('loadedAddresses',{}).get('readonly',[])
                instructions = message['instructions'] + [i for group in tx['meta'].get('innerInstructions',[]) for i in group['instructions']]
                for ix in instructions:
                    program = keys[ix['programIdIndex']]
                    position = POOL_INDICES.get(program)
                    if position is None or len(ix['accounts']) <= position:
                        continue
                    pool = keys[ix['accounts'][position]]
                    entry = manifest['pools'].setdefault(pool, {'program_id':program,'signatures':[], 'referenced_stock_mints':[]})
                    if signature not in entry['signatures']:
                        entry['signatures'].append(signature)
                    entry['referenced_stock_mints'] = sorted(set(entry['referenced_stock_mints']) | mints)
            if count % 100 == 0:
                print(f'fetched {count}/{len(signatures)}; candidate pools={len(manifest["pools"])}', flush=True)
                (args.output/'manifest.json').write_text(json.dumps(manifest, indent=2))
    manifest['transactions_fetched'] = count
    manifest['official_mints_checked'] = len(official)
    manifest['requested_successful_signatures'] = len(signatures)
    manifest['completed_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    manifest['identity_records'] = {m:official[m] for m in manifest['matched_mints']}
    (args.output/'manifest.json').write_text(json.dumps(manifest, indent=2))
    print(json.dumps({'fetched':count,'gaps':len(manifest['fetch_failures']),
                      'source_failures':len(manifest['source_failures']),
                      'matched_mints':len(manifest['matched_mints']),'candidate_pools':len(manifest['pools'])}))
    if not signatures:
        raise SystemExit('No successful signatures available; see source_failures in manifest.json')


if __name__ == '__main__':
    main()
