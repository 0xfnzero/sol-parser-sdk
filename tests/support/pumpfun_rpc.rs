// Reconstruct the signed Solana wire transaction from an unparsed getTransaction JSON fixture.
// This preserves original signatures, instruction data/account indices and address table lookups.
pub fn wire(raw: &serde_json::Value) -> Vec<u8> {
    fn short(out: &mut Vec<u8>, mut n: usize) {
        loop {
            let mut b = (n & 127) as u8;
            n >>= 7;
            if n > 0 {
                b |= 128;
            }
            out.push(b);
            if n == 0 {
                break;
            }
        }
    }
    fn key(out: &mut Vec<u8>, s: &serde_json::Value) {
        out.extend(bs58::decode(s.as_str().unwrap()).into_vec().unwrap());
    }
    let tx = &raw["transaction"];
    let msg = &tx["message"];
    let mut out = vec![];
    let sigs = tx["signatures"].as_array().unwrap();
    short(&mut out, sigs.len());
    for sig in sigs {
        key(&mut out, sig);
    }
    if raw["version"] == 0 {
        out.push(128);
    }
    for f in ["numRequiredSignatures", "numReadonlySignedAccounts", "numReadonlyUnsignedAccounts"] {
        out.push(msg["header"][f].as_u64().unwrap() as u8);
    }
    let keys = msg["accountKeys"].as_array().unwrap();
    short(&mut out, keys.len());
    for k in keys {
        key(&mut out, k);
    }
    key(&mut out, &msg["recentBlockhash"]);
    let ixs = msg["instructions"].as_array().unwrap();
    short(&mut out, ixs.len());
    for ix in ixs {
        out.push(ix["programIdIndex"].as_u64().unwrap() as u8);
        let accounts = ix["accounts"].as_array().unwrap();
        short(&mut out, accounts.len());
        for a in accounts {
            out.push(a.as_u64().unwrap() as u8);
        }
        let data = bs58::decode(ix["data"].as_str().unwrap()).into_vec().unwrap();
        short(&mut out, data.len());
        out.extend(data);
    }
    if raw["version"] == 0 {
        let lookups = msg["addressTableLookups"].as_array().unwrap();
        short(&mut out, lookups.len());
        for l in lookups {
            key(&mut out, &l["accountKey"]);
            for f in ["writableIndexes", "readonlyIndexes"] {
                let indices = l[f].as_array().unwrap();
                short(&mut out, indices.len());
                for i in indices {
                    out.push(i.as_u64().unwrap() as u8);
                }
            }
        }
    }
    out
}
