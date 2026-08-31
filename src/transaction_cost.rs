//! Opt-in transaction fee, compute-budget, and relay-tip parsing.
//!
//! These functions are intentionally separate from DEX event parsing. Callers
//! that do not request transaction costs incur no instruction scan.

use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey, transaction::VersionedTransaction};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use yellowstone_grpc_proto::prelude::{InnerInstruction, Transaction, TransactionStatusMeta};

use crate::rpc_parser::{convert_rpc_to_grpc, ParseError};

const COMPUTE_BUDGET_PROGRAM_ID: Pubkey = pubkey!("ComputeBudget111111111111111111111111111111");
const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
const MAX_COMPUTE_UNIT_LIMIT: u32 = 1_400_000;
const MICRO_LAMPORTS_PER_LAMPORT: u128 = 1_000_000;
const SET_COMPUTE_UNIT_LIMIT_TAG: u8 = 2;
const SET_COMPUTE_UNIT_PRICE_TAG: u8 = 3;
const REQUEST_HEAP_FRAME_TAG: u8 = 1;
const SET_LOADED_ACCOUNTS_DATA_SIZE_LIMIT_TAG: u8 = 4;
const SYSTEM_TRANSFER_TAG: [u8; 4] = 2u32.to_le_bytes();

/// SWQoS providers supported by `sol-trade-sdk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwqosProvider {
    Jito,
    NextBlock,
    ZeroSlot,
    Temporal,
    Bloxroute,
    Node1,
    FlashBlock,
    BlockRazor,
    Astralane,
    Stellium,
    Lightspeed,
    Soyas,
    Speedlanding,
    Helius,
    Solami,
    LunarLander,
    Glaive,
}

/// A provider and all tip accounts accepted by that provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwqosTipAccountGroup {
    pub provider: SwqosProvider,
    pub accounts: &'static [Pubkey],
}

/// Official Jito block-engine tip accounts used by `sol-trade-sdk`.
pub const JITO_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"),
    pubkey!("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe"),
    pubkey!("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"),
    pubkey!("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49"),
    pubkey!("DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh"),
    pubkey!("ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt"),
    pubkey!("DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL"),
    pubkey!("3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT"),
];

pub const HELIUS_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("4ACfpUFoaSD9bfPdeu6DBt89gB6ENTeHBXCAi87NhDEE"),
    pubkey!("D2L6yPZ2FmmmTKPgzaMKdhu6EWZcTpLy1Vhx8uvZe7NZ"),
    pubkey!("9bnz4RShgq1hAnLnZbP8kbgBg1kEmcJBYQq3gQbmnSta"),
    pubkey!("5VY91ws6B2hMmBFRsXkoAAdsPHBJwRfBht4DXox3xkwn"),
    pubkey!("2nyhqdwKcJZR2vcqCyrYsaPVdAnFoJjiksCXJ7hfEYgD"),
    pubkey!("2q5pghRs6arqVjRvT5gfgWfWcHWmw1ZuCzphgd5KfWGJ"),
    pubkey!("wyvPkWjVZz1M8fHQnMMCDTQDbkManefNNhweYk5WkcF"),
    pubkey!("3KCKozbAaF75qEU33jtzozcJ29yJuaLJTy2jFdzUY8bT"),
    pubkey!("4vieeGHPYPG2MmyPRcYjdiDmmhN3ww7hsFNap8pVN3Ey"),
    pubkey!("4TQLFNWK8AovT1gFvda5jfw2oJeRMKEmw7aH6MGBJ3or"),
];

pub const NEXTBLOCK_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("NextbLoCkVtMGcV47JzewQdvBpLqT9TxQFozQkN98pE"),
    pubkey!("NexTbLoCkWykbLuB1NkjXgFWkX9oAtcoagQegygXXA2"),
    pubkey!("NeXTBLoCKs9F1y5PJS9CKrFNNLU1keHW71rfh7KgA1X"),
    pubkey!("NexTBLockJYZ7QD7p2byrUa6df8ndV2WSd8GkbWqfbb"),
    pubkey!("neXtBLock1LeC67jYd1QdAa32kbVeubsfPNTJC1V5At"),
    pubkey!("nEXTBLockYgngeRmRrjDV31mGSekVPqZoMGhQEZtPVG"),
    pubkey!("NEXTbLoCkB51HpLBLojQfpyVAMorm3zzKg7w9NFdqid"),
    pubkey!("nextBLoCkPMgmG8ZgJtABeScP35qLa2AMCNKntAP7Xc"),
];

pub const ZEROSLOT_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("Eb2KpSC8uMt9GmzyAEm5Eb1AAAgTjRaXWFjKyFXHZxF3"),
    pubkey!("FCjUJZ1qozm1e8romw216qyfQMaaWKxWsuySnumVCCNe"),
    pubkey!("ENxTEjSQ1YabmUpXAdCgevnHQ9MHdLv8tzFiuiYJqa13"),
    pubkey!("6rYLG55Q9RpsPGvqdPNJs4z5WTxJVatMB8zV3WJhs5EK"),
    pubkey!("Cix2bHfqPcKcM233mzxbLk14kSggUUiz2A87fJtGivXr"),
];

pub const NOZOMI_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("TEMPaMeCRFAS9EKF53Jd6KpHxgL47uWLcpFArU1Fanq"),
    pubkey!("noz3jAjPiHuBPqiSPkkugaJDkJscPuRhYnSpbi8UvC4"),
    pubkey!("noz3str9KXfpKknefHji8L1mPgimezaiUyCHYMDv1GE"),
    pubkey!("noz6uoYCDijhu1V7cutCpwxNiSovEwLdRHPwmgCGDNo"),
    pubkey!("noz9EPNcT7WH6Sou3sr3GGjHQYVkN3DNirpbvDkv9YJ"),
    pubkey!("nozc5yT15LazbLTFVZzoNZCwjh3yUtW86LoUyqsBu4L"),
    pubkey!("nozFrhfnNGoyqwVuwPAW4aaGqempx4PU6g6D9CJMv7Z"),
    pubkey!("nozievPk7HyK1Rqy1MPJwVQ7qQg2QoJGyP71oeDwbsu"),
    pubkey!("noznbgwYnBLDHu8wcQVCEw6kDrXkPdKkydGJGNXGvL7"),
    pubkey!("nozNVWs5N8mgzuD3qigrCG2UoKxZttxzZ85pvAQVrbP"),
    pubkey!("nozpEGbwx4BcGp6pvEdAh1JoC2CQGZdU6HbNP1v2p6P"),
    pubkey!("nozrhjhkCr3zXT3BiT4WCodYCUFeQvcdUkM7MqhKqge"),
    pubkey!("nozrwQtWhEdrA6W8dkbt9gnUaMs52PdAv5byipnadq3"),
    pubkey!("nozUacTVWub3cL4mJmGCYjKZTnE9RbdY5AP46iQgbPJ"),
    pubkey!("nozWCyTPppJjRuw2fpzDhhWbW355fzosWSzrrMYB1Qk"),
    pubkey!("nozWNju6dY353eMkMqURqwQEoM3SFgEKC6psLCSfUne"),
    pubkey!("nozxNBgWohjR75vdspfxR5H9ceC7XXH99xpxhVGt3Bb"),
];

pub const BLOX_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY"),
    pubkey!("95cfoy472fcQHaw4tPGBTKpn6ZQnfEPfBgDQx6gcRmRg"),
    pubkey!("3UQUKjhMKaY2S6bjcQD6yHB7utcZt5bfarRCmctpRtUd"),
    pubkey!("FogxVNs6Mm2w9rnGL1vkARSwJxvLE8mujTv3LK8RnUhF"),
];

pub const NODE1_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("node1PqAa3BWWzUnTHVbw8NJHC874zn9ngAkXjgWEej"),
    pubkey!("node1UzzTxAAeBTpfZkQPJXBAqixsbdth11ba1NXLBG"),
    pubkey!("node1Qm1bV4fwYnCurP8otJ9s5yrkPq7SPZ5uhj3Tsv"),
    pubkey!("node1PUber6SFmSQgvf2ECmXsHP5o3boRSGhvJyPMX1"),
    pubkey!("node1AyMbeqiVN6eoQzEAwCA6Pk826hrdqdAHR7cdJ3"),
    pubkey!("node1YtWCoTwwVYTFLfS19zquRQzYX332hs1HEuRBjC"),
];

pub const FLASHBLOCK_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("FLaShB3iXXTWE1vu9wQsChUKq3HFtpMAhb8kAh1pf1wi"),
    pubkey!("FLashhsorBmM9dLpuq6qATawcpqk1Y2aqaZfkd48iT3W"),
    pubkey!("FLaSHJNm5dWYzEgnHJWWJP5ccu128Mu61NJLxUf7mUXU"),
    pubkey!("FLaSHR4Vv7sttd6TyDF4yR1bJyAxRwWKbohDytEMu3wL"),
    pubkey!("FLASHRzANfcAKDuQ3RXv9hbkBy4WVEKDzoAgxJ56DiE4"),
    pubkey!("FLasHstqx11M8W56zrSEqkCyhMCCpr6ze6Mjdvqope5s"),
    pubkey!("FLAShWTjcweNT4NSotpjpxAkwxUr2we3eXQGhpTVzRwy"),
    pubkey!("FLasHXTqrbNvpWFB6grN47HGZfK6pze9HLNTgbukfPSk"),
    pubkey!("FLAshyAyBcKb39KPxSzXcepiS8iDYUhDGwJcJDPX4g2B"),
    pubkey!("FLAsHZTRcf3Dy1APaz6j74ebdMC6Xx4g6i9YxjyrDybR"),
];

pub const BLOCKRAZOR_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("FjmZZrFvhnqqb9ThCuMVnENaM3JGVuGWNyCAxRJcFpg9"),
    pubkey!("6No2i3aawzHsjtThw81iq1EXPJN6rh8eSJCLaYZfKDTG"),
    pubkey!("A9cWowVAiHe9pJfKAj3TJiN9VpbzMUq6E4kEvf5mUT22"),
    pubkey!("Gywj98ophM7GmkDdaWs4isqZnDdFCW7B46TXmKfvyqSm"),
    pubkey!("68Pwb4jS7eZATjDfhmTXgRJjCiZmw1L7Huy4HNpnxJ3o"),
    pubkey!("4ABhJh5rZPjv63RBJBuyWzBK3g9gWMUQdTZP2kiW31V9"),
    pubkey!("B2M4NG5eyZp5SBQrSdtemzk5TqVuaWGQnowGaCBt8GyM"),
    pubkey!("5jA59cXMKQqZAVdtopv8q3yyw9SYfiE3vUCbt7p8MfVf"),
    pubkey!("5YktoWygr1Bp9wiS1xtMtUki1PeYuuzuCF98tqwYxf61"),
    pubkey!("295Avbam4qGShBYK7E9H5Ldew4B3WyJGmgmXfiWdeeyV"),
    pubkey!("EDi4rSy2LZgKJX74mbLTFk4mxoTgT6F7HxxzG2HBAFyK"),
    pubkey!("BnGKHAC386n4Qmv9xtpBVbRaUTKixjBe3oagkPFKtoy6"),
    pubkey!("Dd7K2Fp7AtoN8xCghKDRmyqr5U169t48Tw5fEd3wT9mq"),
    pubkey!("AP6qExwrbRgBAVaehg4b5xHENX815sMabtBzUzVB4v8S"),
];

pub const ASTRALANE_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("astrazznxsGUhWShqgNtAdfrzP2G83DzcWVJDxwV9bF"),
    pubkey!("astra4uejePWneqNaJKuFFA8oonqCE1sqF6b45kDMZm"),
    pubkey!("astra9xWY93QyfG6yM8zwsKsRodscjQ2uU2HKNL5prk"),
    pubkey!("astraRVUuTHjpwEVvNBeQEgwYx9w9CFyfxjYoobCZhL"),
    pubkey!("astraEJ2fEj8Xmy6KLG7B3VfbKfsHXhHrNdCQx7iGJK"),
    pubkey!("astraubkDw81n4LuutzSQ8uzHCv4BhPVhfvTcYv8SKC"),
    pubkey!("astraZW5GLFefxNPAatceHhYjfA1ciq9gvfEg2S47xk"),
    pubkey!("astrawVNP4xDBKT7rAdxrLYiTSTdqtUr63fSMduivXK"),
    pubkey!("AstrA1ejL4UeXC2SBP4cpeEmtcFPZVLxx3XGKXyCW6to"),
    pubkey!("AsTra79FET4aCKWspPqeSFvjJNyp96SvAnrmyAxqg5b7"),
    pubkey!("AstrABAu8CBTyuPXpV4eSCJ5fePEPnxN8NqBaPKQ9fHR"),
    pubkey!("AsTRADtvb6tTmrsqULQ9Wji9PigDMjhfEMza6zkynEvV"),
    pubkey!("AsTRAEoyMofR3vUPpf9k68Gsfb6ymTZttEtsAbv8Bk4d"),
    pubkey!("AStrAJv2RN2hKCHxwUMtqmSxgdcNZbihCwc1mCSnG83W"),
    pubkey!("Astran35aiQUF57XZsmkWMtNCtXGLzs8upfiqXxth2bz"),
    pubkey!("AStRAnpi6kFrKypragExgeRoJ1QnKH7pbSjLAKQVWUum"),
    pubkey!("ASTRaoF93eYt73TYvwtsv6fMWHWbGmMUZfVZPo3CRU9C"),
];

pub const STELLIUM_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("ste11JV3MLMM7x7EJUM2sXcJC1H7F4jBLnP9a9PG8PH"),
    pubkey!("ste11MWPjXCRfQryCshzi86SGhuXjF4Lv6xMXD2AoSt"),
    pubkey!("ste11p5x8tJ53H1NbNQsRBg1YNRd4GcVpxtDw8PBpmb"),
    pubkey!("ste11p7e2KLYou5bwtt35H7BM6uMdo4pvioGjJXKFcN"),
    pubkey!("ste11TMV68LMi1BguM4RQujtbNCZvf1sjsASpqgAvSX"),
];

pub const LIGHTSPEED_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("53PhM3UTdMQWu5t81wcd35AHGc5xpmHoRjem7GQPvXjA"),
    pubkey!("9tYF5yPDC1NP8s6diiB3kAX6ZZnva9DM3iDwJkBRarBB"),
];

pub const SOYAS_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("soyas4s6L8KWZ8rsSk1mF3d1mQScoTGGAgjk98bF8nP"),
    pubkey!("soyascXFW5wEEYiwfEmHy2pNwomqzvggJosGVD6TJdY"),
    pubkey!("soyasDBdKjADwPz3xk82U3TNPRDKEWJj7wWLajNHZ1L"),
    pubkey!("soyasE2abjBAynmHbGWgEwk4ctBy7JMTUCNrMbjcnyH"),
    pubkey!("soyasi59njacMUPvo3TM5paHjeK8pYSdovXgFi32gRt"),
    pubkey!("soyasQYhJxv8uZgWDxhg72td6piAf7XTkoyWHtSATEz"),
    pubkey!("soyastP66xyYC8XADXZjdMM5BAVGD2YRvz8dwtLsqb8"),
    pubkey!("soyasvdgUJWYcUCzDxpmjUnNjH7KamXLXTzLwFvdVPE"),
    pubkey!("soyasvxAunisNxaoRxkKGjNir7KmbwYnr37JmefkX9G"),
    pubkey!("soyas5doVFUwH8s5zK8gEvCL5KR5ogDmf52LsrJEZ9h"),
];

pub const SPEEDLANDING_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("SpEEdz8S1KorkMZqjMUxfxrmWwofmp6ReNP2Nx6CUmq"),
    pubkey!("SpeeDy3GJM4wcrQmk1itRFWgidvxX4rwjTLMv78wwjE"),
    pubkey!("SPeEdva37vW8vRtqgYjprQs1g3965icfVN5Rt7SMAyh"),
    pubkey!("speEdrSEpox5GUfHWcBc7tQjRuSfUin2yvB7qoYvvJh"),
    pubkey!("SPeEDmkHkN3A2roSZf6aZyEMsmrGqTHKqwP51y2Y4rV"),
    pubkey!("SpeedLdTJXh2RKpXEaP8JCxkWoUVXhtdPQ1EnxBJMxc"),
    pubkey!("SpEediGKLbbXndSYTzwmz6Z3NDgHQLDcTDEvGFkSMH9"),
    pubkey!("speede8xCcUq2Tiv1efXeTuE3k9TDNq8TnGKaKSc6J4"),
];

pub const SOLAMI_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("15qWd4huAkoxvhDsHMfpUn27TW1YBYMMJJ2jkAkbeam"),
    pubkey!("9XuGciSwr5wb7dLTQm91JhuBTvj3GG8WjuRDc3obeam"),
    pubkey!("kiQioJNyFG7pU36ELLsRKXkeT48kFbk3b6rSgrWbeam"),
    pubkey!("kjmVhW1UzJrW2sU5bY5NtZ79jpvjSStsj37Pzmabeam"),
    pubkey!("kREnjPWFpt4AHeY5pijPmyXaCrMnbatUQJo7d3Xbeam"),
    pubkey!("praRZG6N6MdbsT4EFpKgZJWReZGXQhAMFcH68oCbeam"),
    pubkey!("SqoKQKU5uwBxovq3R7yEBxFwptc4z7vwoghU3M9beam"),
    pubkey!("sV72TY66T1RfmDSeHPPbwX6wwJ3bBv5hd4ehJ8tbeam"),
    pubkey!("swf8MyEeLo7gtRUo27UuJj6naCASUrypU7dbteSbeam"),
    pubkey!("uiuaQsxA47JybQAVN4FTfYuoEDkMiXV1r591Aewbeam"),
];

pub const LUNARLANDER_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("moon17L6BgxXRX5uHKudAmqVF96xia9h8ygcmG2sL3F"),
    pubkey!("moon26Sek222Md7ZydcAGxoKG832DK36CkLrS3PQY4c"),
    pubkey!("moon7fwyajcVstMoBnVy7UBcTx87SBtNoGGAaH2Cb8V"),
    pubkey!("moonBtH9HvLHjLqi9ivyrMVKgFUsSfrz9BwQ9khhn1u"),
    pubkey!("moonCJg8476LNFLptX1qrK8PdRsA1HD1R6XWyu9MB93"),
    pubkey!("moonF2sz7qwAtdETnrgxNbjonnhGGjd6r4W4UC9284s"),
    pubkey!("moonKfftMiGSak3cezvhEqvkPSzwrmQxQHXuspC96yj"),
    pubkey!("moonQBUKBpkifLcTd78bfxxt4PYLwmJ5admLW6cBBs8"),
    pubkey!("moonXwpKwoVkMegt5Bc776cSW793X1irL5hHV1vJ3JA"),
    pubkey!("moonZ6u9E2fgk6eWd82621eLPHt9zuJuYECXAYjMY1C"),
];

pub const GLAIVE_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("GLaiv4GMRYQmthatDS98uQT4HoucgxWT8NeJz6oSwxeU"),
    pubkey!("GLaivL5uPrDpvd1wTtvat38KGqb5WLhEdqQfnmNd3oNr"),
    pubkey!("GLaivinAWh21NaJMhtExtD5G2gZs1xnvaYVZmwqobWZL"),
    pubkey!("GLaivJSUL71FcocYa8tks5vpVyYzvaDMHtyrzfQF2ABr"),
    pubkey!("GLaivRU6eDKrta3p3psFAWPEFLzCjeMHGpPUuQqTjtyv"),
    pubkey!("GLaivq5dU8qHayz9Qf13LjPfVy3SmUhbmickfGiZdmfh"),
];

/// Complete provider-aware tip registry mirrored from `sol-trade-sdk`.
pub const SWQOS_TIP_ACCOUNT_GROUPS: &[SwqosTipAccountGroup] = &[
    SwqosTipAccountGroup { provider: SwqosProvider::Jito, accounts: JITO_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::NextBlock, accounts: NEXTBLOCK_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::ZeroSlot, accounts: ZEROSLOT_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Temporal, accounts: NOZOMI_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Bloxroute, accounts: BLOX_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Node1, accounts: NODE1_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::FlashBlock, accounts: FLASHBLOCK_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::BlockRazor, accounts: BLOCKRAZOR_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Astralane, accounts: ASTRALANE_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Stellium, accounts: STELLIUM_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Lightspeed, accounts: LIGHTSPEED_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Soyas, accounts: SOYAS_TIP_ACCOUNTS },
    SwqosTipAccountGroup {
        provider: SwqosProvider::Speedlanding,
        accounts: SPEEDLANDING_TIP_ACCOUNTS,
    },
    SwqosTipAccountGroup { provider: SwqosProvider::Helius, accounts: HELIUS_TIP_ACCOUNTS },
    SwqosTipAccountGroup { provider: SwqosProvider::Solami, accounts: SOLAMI_TIP_ACCOUNTS },
    SwqosTipAccountGroup {
        provider: SwqosProvider::LunarLander,
        accounts: LUNARLANDER_TIP_ACCOUNTS,
    },
    SwqosTipAccountGroup { provider: SwqosProvider::Glaive, accounts: GLAIVE_TIP_ACCOUNTS },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TipPayment {
    pub provider: SwqosProvider,
    pub source: Pubkey,
    pub recipient: Pubkey,
    pub lamports: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TransactionCost {
    /// Authoritative transaction fee from status metadata, including priority fee.
    pub transaction_fee_lamports: Option<u64>,
    /// Sum of transaction fee and recognized relay tips.
    pub total_fee_and_tip_lamports: Option<u64>,
    pub compute_units_consumed: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub compute_unit_price_micro_lamports: Option<u64>,
    /// Requested priority fee, rounded up with Solana runtime semantics.
    pub priority_fee_lamports: Option<u64>,
    /// `true` when status metadata confirms the transaction succeeded.
    pub tip_payments_confirmed: bool,
    pub tip_lamports: u64,
    pub tip_payments: Vec<TipPayment>,
}

impl TransactionCost {
    /// Returns the total tip paid to a specific provider.
    #[inline]
    pub fn tip_lamports_for(&self, provider: SwqosProvider) -> u64 {
        self.tip_payments
            .iter()
            .filter(|payment| payment.provider == provider)
            .fold(0u64, |total, payment| total.saturating_add(payment.lamports))
    }
}

#[derive(Default)]
struct ScanState {
    compute_unit_limit: Option<u32>,
    compute_unit_price_micro_lamports: Option<u64>,
    direct_priority_fee_lamports: Option<u64>,
    is_v1: bool,
    seen_compute_budget_tags: u8,
    invalid_compute_budget: bool,
    tip_lamports: u64,
    tip_payments: Vec<TipPayment>,
}

impl ScanState {
    #[inline]
    fn with_v1_fields(compute_unit_limit: Option<u32>, priority_fee: Option<u64>) -> Self {
        Self {
            compute_unit_limit,
            direct_priority_fee_lamports: priority_fee,
            is_v1: true,
            ..Self::default()
        }
    }

    #[inline]
    fn finish(
        mut self,
        transaction_fee_lamports: Option<u64>,
        compute_units_consumed: Option<u64>,
        tip_payments_confirmed: bool,
    ) -> TransactionCost {
        if self.invalid_compute_budget {
            self.compute_unit_limit = None;
            self.compute_unit_price_micro_lamports = None;
        }
        let priority_fee_lamports = self.direct_priority_fee_lamports.or_else(|| {
            self.compute_unit_limit.zip(self.compute_unit_price_micro_lamports).map(
                |(limit, price)| priority_fee_lamports(limit.min(MAX_COMPUTE_UNIT_LIMIT), price),
            )
        });
        TransactionCost {
            transaction_fee_lamports,
            total_fee_and_tip_lamports: transaction_fee_lamports
                .map(|fee| fee.saturating_add(self.tip_lamports)),
            compute_units_consumed,
            compute_unit_limit: self.compute_unit_limit,
            compute_unit_price_micro_lamports: self.compute_unit_price_micro_lamports,
            priority_fee_lamports,
            tip_payments_confirmed,
            tip_lamports: self.tip_lamports,
            tip_payments: self.tip_payments,
        }
    }
}

/// Parses costs from a binary-encoded RPC transaction response.
pub fn parse_rpc_transaction_cost(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<TransactionCost, ParseError> {
    let (meta, transaction) = convert_rpc_to_grpc(transaction)?;
    parse_yellowstone_transaction_cost(&transaction, &meta)
        .ok_or_else(|| ParseError::MissingField("transaction.message".to_string()))
}

/// Parses costs from a Yellowstone transaction, including ALT-loaded accounts
/// and inner System Program transfers. Returns `None` when the transaction
/// message is absent.
pub fn parse_yellowstone_transaction_cost(
    transaction: &Transaction,
    meta: &TransactionStatusMeta,
) -> Option<TransactionCost> {
    let message = transaction.message.as_ref()?;
    let get_key = |index: usize| {
        let static_len = message.account_keys.len();
        if index < static_len {
            return pubkey_from_bytes(message.account_keys.get(index)?);
        }
        let writable_index = index - static_len;
        if writable_index < meta.loaded_writable_addresses.len() {
            return pubkey_from_bytes(meta.loaded_writable_addresses.get(writable_index)?);
        }
        let readonly_index = writable_index - meta.loaded_writable_addresses.len();
        pubkey_from_bytes(meta.loaded_readonly_addresses.get(readonly_index)?)
    };

    let mut state = message
        .config
        .as_ref()
        .map(|config| ScanState::with_v1_fields(config.compute_unit_limit, config.priority_fee))
        .unwrap_or_default();
    for instruction in &message.instructions {
        scan_instruction(
            instruction.program_id_index as usize,
            &instruction.accounts,
            &instruction.data,
            &get_key,
            meta.err.is_none(),
            &mut state,
        );
    }
    if meta.err.is_none() {
        for group in &meta.inner_instructions {
            for instruction in &group.instructions {
                scan_inner_tip(instruction, &get_key, &mut state);
            }
        }
    }

    Some(state.finish(Some(meta.fee), meta.compute_units_consumed, meta.err.is_none()))
}

/// Parses requested compute budget and outer relay tips from a ShredStream
/// transaction. Transaction status metadata and ALT-loaded addresses are not
/// present in shred entries, so fee and consumed-CU fields remain `None` and
/// tips whose recipient is ALT-loaded cannot be recognized.
pub fn parse_shred_transaction_cost(transaction: &VersionedTransaction) -> TransactionCost {
    let message = &transaction.message;
    let static_keys = message.static_account_keys();
    let get_key = |index: usize| static_keys.get(index).copied();
    let mut state = match message {
        solana_sdk::message::VersionedMessage::V1(message) => ScanState::with_v1_fields(
            message.config.compute_unit_limit,
            message.config.priority_fee,
        ),
        solana_sdk::message::VersionedMessage::Legacy(_)
        | solana_sdk::message::VersionedMessage::V0(_) => ScanState::default(),
    };
    for instruction in message.instructions() {
        scan_instruction(
            instruction.program_id_index as usize,
            &instruction.accounts,
            &instruction.data,
            &get_key,
            true,
            &mut state,
        );
    }
    state.finish(None, None, false)
}

#[inline]
fn scan_inner_tip<F>(instruction: &InnerInstruction, get_key: &F, state: &mut ScanState)
where
    F: Fn(usize) -> Option<Pubkey>,
{
    let Some(program_id) = get_key(instruction.program_id_index as usize) else {
        return;
    };
    if program_id == SYSTEM_PROGRAM_ID {
        scan_system_transfer(&instruction.accounts, &instruction.data, get_key, state);
    }
}

#[inline]
fn scan_instruction<F>(
    program_id_index: usize,
    accounts: &[u8],
    data: &[u8],
    get_key: &F,
    scan_tips: bool,
    state: &mut ScanState,
) where
    F: Fn(usize) -> Option<Pubkey>,
{
    let Some(program_id) = get_key(program_id_index) else {
        return;
    };
    if program_id == COMPUTE_BUDGET_PROGRAM_ID && !state.is_v1 {
        scan_compute_budget(data, state);
    } else if scan_tips && program_id == SYSTEM_PROGRAM_ID {
        scan_system_transfer(accounts, data, get_key, state);
    }
}

#[inline]
fn scan_compute_budget(data: &[u8], state: &mut ScanState) {
    let Some(tag) = data.first().copied() else {
        state.invalid_compute_budget = true;
        return;
    };
    if tag > SET_LOADED_ACCOUNTS_DATA_SIZE_LIMIT_TAG {
        state.invalid_compute_budget = true;
        return;
    }
    let tag_mask = 1u8 << tag;
    if state.seen_compute_budget_tags & tag_mask != 0 {
        state.invalid_compute_budget = true;
        return;
    }
    state.seen_compute_budget_tags |= tag_mask;

    match tag {
        SET_COMPUTE_UNIT_LIMIT_TAG => {
            let Some(value) = read_u32_exact(data) else {
                state.invalid_compute_budget = true;
                return;
            };
            state.compute_unit_limit = Some(value);
        }
        SET_COMPUTE_UNIT_PRICE_TAG => {
            let Some(value) = read_u64_exact(data) else {
                state.invalid_compute_budget = true;
                return;
            };
            state.compute_unit_price_micro_lamports = Some(value);
        }
        REQUEST_HEAP_FRAME_TAG | SET_LOADED_ACCOUNTS_DATA_SIZE_LIMIT_TAG if data.len() == 5 => {}
        _ => state.invalid_compute_budget = true,
    }
}

#[inline]
fn scan_system_transfer<F>(accounts: &[u8], data: &[u8], get_key: &F, state: &mut ScanState)
where
    F: Fn(usize) -> Option<Pubkey>,
{
    if data.len() != 12 || data.get(..4) != Some(SYSTEM_TRANSFER_TAG.as_slice()) {
        return;
    }
    let Some(source) = accounts.first().and_then(|index| get_key(*index as usize)) else {
        return;
    };
    let Some(recipient) = accounts.get(1).and_then(|index| get_key(*index as usize)) else {
        return;
    };
    let Some(provider) = find_tip_provider(&recipient) else {
        return;
    };
    let lamports = u64::from_le_bytes(data[4..12].try_into().expect("validated transfer length"));
    state.tip_lamports = state.tip_lamports.saturating_add(lamports);
    state.tip_payments.push(TipPayment { provider, source, recipient, lamports });
}

#[inline]
fn find_tip_provider(recipient: &Pubkey) -> Option<SwqosProvider> {
    SWQOS_TIP_ACCOUNT_GROUPS
        .iter()
        .find(|group| group.accounts.contains(recipient))
        .map(|group| group.provider)
}

#[inline]
fn priority_fee_lamports(compute_unit_limit: u32, micro_lamports: u64) -> u64 {
    let fee = (compute_unit_limit as u128).saturating_mul(micro_lamports as u128);
    fee.saturating_add(MICRO_LAMPORTS_PER_LAMPORT - 1)
        .checked_div(MICRO_LAMPORTS_PER_LAMPORT)
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(u64::MAX)
}

#[inline]
fn read_u32_exact(data: &[u8]) -> Option<u32> {
    (data.len() == 5).then(|| u32::from_le_bytes(data[1..5].try_into().unwrap()))
}

#[inline]
fn read_u64_exact(data: &[u8]) -> Option<u64> {
    (data.len() == 9).then(|| u64::from_le_bytes(data[1..9].try_into().unwrap()))
}

#[inline]
fn pubkey_from_bytes(bytes: &[u8]) -> Option<Pubkey> {
    Pubkey::try_from(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use solana_client::rpc_response::UiTransactionError;
    use solana_sdk::{
        hash::Hash,
        message::{
            compiled_instruction::CompiledInstruction as SdkInstruction, v0, v1, MessageHeader,
            VersionedMessage,
        },
        signature::Signature,
    };
    use solana_transaction_status::{
        option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta,
        EncodedTransaction, EncodedTransactionWithStatusMeta, TransactionBinaryEncoding,
        UiTransactionStatusMeta,
    };
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, InnerInstructions, Message, TransactionError, TransactionStatusMeta,
    };

    fn compute_limit(units: u32) -> Vec<u8> {
        let mut data = vec![SET_COMPUTE_UNIT_LIMIT_TAG];
        data.extend_from_slice(&units.to_le_bytes());
        data
    }

    fn compute_price(micro_lamports: u64) -> Vec<u8> {
        let mut data = vec![SET_COMPUTE_UNIT_PRICE_TAG];
        data.extend_from_slice(&micro_lamports.to_le_bytes());
        data
    }

    fn transfer(lamports: u64) -> Vec<u8> {
        let mut data = SYSTEM_TRANSFER_TAG.to_vec();
        data.extend_from_slice(&lamports.to_le_bytes());
        data
    }

    fn grpc_instruction(
        program_id_index: u32,
        accounts: Vec<u8>,
        data: Vec<u8>,
    ) -> CompiledInstruction {
        CompiledInstruction { program_id_index, accounts, data }
    }

    fn v1_transaction() -> VersionedTransaction {
        let source = Pubkey::new_unique();
        VersionedTransaction {
            signatures: vec![Signature::from([7; 64])],
            message: VersionedMessage::V1(v1::Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 2,
                },
                config: v1::TransactionConfig::empty()
                    .with_compute_unit_limit(234_567)
                    .with_priority_fee(4_321),
                lifetime_specifier: Hash::new_unique(),
                account_keys: vec![
                    source,
                    JITO_TIP_ACCOUNTS[0],
                    SYSTEM_PROGRAM_ID,
                    COMPUTE_BUDGET_PROGRAM_ID,
                ],
                instructions: vec![
                    SdkInstruction::new_from_raw_parts(3, compute_limit(999_999), vec![]),
                    SdkInstruction::new_from_raw_parts(3, compute_price(999_999), vec![]),
                    SdkInstruction::new_from_raw_parts(2, transfer(42), vec![0, 1]),
                ],
            }),
        }
    }

    #[test]
    fn yellowstone_parser_reads_fee_compute_budget_and_known_tip() {
        let source = Pubkey::new_unique();
        let ordinary_recipient = Pubkey::new_unique();
        let keys = [source, COMPUTE_BUDGET_PROGRAM_ID, SYSTEM_PROGRAM_ID, ordinary_recipient];
        let transaction = Transaction {
            signatures: vec![vec![0; 64]],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![
                    grpc_instruction(1, vec![], compute_limit(300_000)),
                    grpc_instruction(1, vec![], compute_price(12_345)),
                    grpc_instruction(2, vec![0, 3], transfer(99_999)),
                    grpc_instruction(2, vec![0, 4], transfer(137_273)),
                ],
                ..Default::default()
            }),
        };
        let meta = TransactionStatusMeta {
            fee: 8_704,
            compute_units_consumed: Some(135_026),
            loaded_writable_addresses: vec![JITO_TIP_ACCOUNTS[0].to_bytes().to_vec()],
            ..Default::default()
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &meta).expect("transaction cost");

        assert_eq!(cost.transaction_fee_lamports, Some(8_704));
        assert_eq!(cost.compute_units_consumed, Some(135_026));
        assert_eq!(cost.compute_unit_limit, Some(300_000));
        assert_eq!(cost.compute_unit_price_micro_lamports, Some(12_345));
        assert_eq!(cost.priority_fee_lamports, Some(3_704));
        assert!(cost.tip_payments_confirmed);
        assert_eq!(cost.tip_lamports, 137_273);
        assert_eq!(cost.total_fee_and_tip_lamports, Some(145_977));
        assert_eq!(cost.tip_payments.len(), 1);
        assert_eq!(cost.tip_payments[0].provider, SwqosProvider::Jito);
        assert_eq!(cost.tip_payments[0].recipient, JITO_TIP_ACCOUNTS[0]);
        assert_eq!(cost.tip_lamports_for(SwqosProvider::Jito), 137_273);
    }

    #[test]
    fn legacy_priority_fee_uses_runtime_clamp_but_exposes_requested_limit() {
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: vec![COMPUTE_BUDGET_PROGRAM_ID.to_bytes().to_vec()],
                instructions: vec![
                    grpc_instruction(0, vec![], compute_limit(2_000_000)),
                    grpc_instruction(0, vec![], compute_price(1_000_000)),
                ],
                ..Default::default()
            }),
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &TransactionStatusMeta::default())
                .expect("transaction cost");

        assert_eq!(cost.compute_unit_limit, Some(2_000_000));
        assert_eq!(cost.priority_fee_lamports, Some(1_400_000));
    }

    #[test]
    fn malformed_duplicate_and_out_of_bounds_instructions_do_not_panic_or_misreport() {
        let keys = [COMPUTE_BUDGET_PROGRAM_ID, SYSTEM_PROGRAM_ID];
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![
                    grpc_instruction(0, vec![], compute_limit(100_000)),
                    grpc_instruction(0, vec![], compute_limit(200_000)),
                    grpc_instruction(0, vec![], vec![SET_COMPUTE_UNIT_PRICE_TAG, 1]),
                    grpc_instruction(1, vec![254, 253], transfer(1_000)),
                    grpc_instruction(250, vec![], vec![]),
                ],
                ..Default::default()
            }),
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &TransactionStatusMeta::default())
                .expect("transaction cost");

        assert_eq!(cost.compute_unit_limit, None);
        assert_eq!(cost.compute_unit_price_micro_lamports, None);
        assert_eq!(cost.priority_fee_lamports, None);
        assert_eq!(cost.tip_lamports, 0);
    }

    #[test]
    fn yellowstone_parser_sums_outer_and_inner_tip_payments() {
        let source = Pubkey::new_unique();
        let keys = [source, SYSTEM_PROGRAM_ID, JITO_TIP_ACCOUNTS[0], GLAIVE_TIP_ACCOUNTS[0]];
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![grpc_instruction(1, vec![0, 2], transfer(10))],
                ..Default::default()
            }),
        };
        let meta = TransactionStatusMeta {
            inner_instructions: vec![InnerInstructions {
                index: 0,
                instructions: vec![InnerInstruction {
                    program_id_index: 1,
                    accounts: vec![0, 3],
                    data: transfer(20),
                    stack_height: Some(2),
                }],
            }],
            ..Default::default()
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &meta).expect("transaction cost");

        assert_eq!(cost.tip_lamports, 30);
        assert_eq!(cost.tip_payments.len(), 2);
        assert_eq!(cost.tip_lamports_for(SwqosProvider::Jito), 10);
        assert_eq!(cost.tip_lamports_for(SwqosProvider::Glaive), 20);
    }

    #[test]
    fn failed_transaction_does_not_report_rolled_back_tips() {
        let source = Pubkey::new_unique();
        let keys = [source, SYSTEM_PROGRAM_ID, JITO_TIP_ACCOUNTS[0]];
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![grpc_instruction(1, vec![0, 2], transfer(500))],
                ..Default::default()
            }),
        };
        let meta = TransactionStatusMeta {
            err: Some(TransactionError { err: vec![1] }),
            fee: 5_000,
            ..Default::default()
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &meta).expect("transaction cost");

        assert_eq!(cost.transaction_fee_lamports, Some(5_000));
        assert!(!cost.tip_payments_confirmed);
        assert_eq!(cost.tip_lamports, 0);
        assert!(cost.tip_payments.is_empty());
    }

    #[test]
    fn inner_compute_budget_instruction_does_not_set_transaction_budget() {
        let keys = [COMPUTE_BUDGET_PROGRAM_ID];
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                ..Default::default()
            }),
        };
        let meta = TransactionStatusMeta {
            inner_instructions: vec![InnerInstructions {
                index: 0,
                instructions: vec![InnerInstruction {
                    program_id_index: 0,
                    accounts: vec![],
                    data: compute_limit(999_999),
                    stack_height: Some(2),
                }],
            }],
            ..Default::default()
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &meta).expect("transaction cost");

        assert_eq!(cost.compute_unit_limit, None);
    }

    #[test]
    fn shred_parser_exposes_requested_costs_without_status_metadata() {
        let source = Pubkey::new_unique();
        let keys = vec![source, COMPUTE_BUDGET_PROGRAM_ID, SYSTEM_PROGRAM_ID, JITO_TIP_ACCOUNTS[0]];
        let transaction = VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(v0::Message {
                header: MessageHeader::default(),
                account_keys: keys,
                recent_blockhash: Hash::default(),
                instructions: vec![
                    SdkInstruction::new_from_raw_parts(1, compute_limit(200_000), vec![]),
                    SdkInstruction::new_from_raw_parts(1, compute_price(5_001), vec![]),
                    SdkInstruction::new_from_raw_parts(2, transfer(42), vec![0, 3]),
                ],
                address_table_lookups: vec![],
            }),
        };

        let cost = parse_shred_transaction_cost(&transaction);

        assert_eq!(cost.transaction_fee_lamports, None);
        assert_eq!(cost.compute_units_consumed, None);
        assert!(!cost.tip_payments_confirmed);
        assert_eq!(cost.priority_fee_lamports, Some(1_001));
        assert_eq!(cost.tip_lamports, 42);
    }

    #[test]
    fn v1_shred_cost_uses_inline_config_and_ignores_compute_budget_instructions() {
        let transaction = v1_transaction();
        let bytes = wincode::serialize(&transaction).expect("serialize V1 transaction");
        let decoded: VersionedTransaction =
            wincode::deserialize(&bytes).expect("deserialize V1 transaction");

        assert_eq!(decoded, transaction);
        let cost = parse_shred_transaction_cost(&decoded);
        assert_eq!(cost.compute_unit_limit, Some(234_567));
        assert_eq!(cost.compute_unit_price_micro_lamports, None);
        assert_eq!(cost.priority_fee_lamports, Some(4_321));
        assert_eq!(cost.tip_lamports, 42);
    }

    #[test]
    fn yellowstone_v1_cost_uses_inline_config_and_ignores_compute_budget_instructions() {
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: vec![COMPUTE_BUDGET_PROGRAM_ID.to_bytes().to_vec()],
                instructions: vec![
                    grpc_instruction(0, vec![], compute_limit(999_999)),
                    grpc_instruction(0, vec![], compute_price(999_999)),
                ],
                config: Some(yellowstone_grpc_proto::prelude::TransactionConfig {
                    priority_fee: Some(4_321),
                    compute_unit_limit: Some(234_567),
                    loaded_accounts_data_size_limit: None,
                    heap_size: None,
                }),
                ..Default::default()
            }),
        };

        let cost =
            parse_yellowstone_transaction_cost(&transaction, &TransactionStatusMeta::default())
                .expect("transaction cost");

        assert_eq!(cost.compute_unit_limit, Some(234_567));
        assert_eq!(cost.compute_unit_price_micro_lamports, None);
        assert_eq!(cost.priority_fee_lamports, Some(4_321));
    }

    #[test]
    fn rpc_cost_preserves_v1_inline_config_through_base64_conversion() {
        let bytes = wincode::serialize(&v1_transaction()).expect("serialize V1 transaction");
        let transaction = EncodedConfirmedTransactionWithStatusMeta {
            slot: 42,
            transaction: EncodedTransactionWithStatusMeta {
                transaction: EncodedTransaction::Binary(
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                    TransactionBinaryEncoding::Base64,
                ),
                meta: Some(UiTransactionStatusMeta {
                    err: None,
                    status: Ok(()),
                    fee: 9_321,
                    pre_balances: vec![],
                    post_balances: vec![],
                    inner_instructions: OptionSerializer::None,
                    log_messages: OptionSerializer::None,
                    pre_token_balances: OptionSerializer::None,
                    post_token_balances: OptionSerializer::None,
                    rewards: OptionSerializer::None,
                    loaded_addresses: OptionSerializer::None,
                    return_data: OptionSerializer::None,
                    compute_units_consumed: OptionSerializer::Some(123_456),
                    cost_units: OptionSerializer::None,
                }),
                version: None,
            },
            block_time: None,
            transaction_index: None,
        };

        let cost = parse_rpc_transaction_cost(&transaction).expect("parse V1 RPC cost");
        assert_eq!(cost.transaction_fee_lamports, Some(9_321));
        assert_eq!(cost.compute_units_consumed, Some(123_456));
        assert_eq!(cost.compute_unit_limit, Some(234_567));
        assert_eq!(cost.compute_unit_price_micro_lamports, None);
        assert_eq!(cost.priority_fee_lamports, Some(4_321));
        assert_eq!(cost.tip_lamports, 42);
        assert_eq!(cost.total_fee_and_tip_lamports, Some(9_363));
    }

    #[test]
    fn failed_rpc_transaction_does_not_confirm_rolled_back_tip() {
        let bytes = wincode::serialize(&v1_transaction()).expect("serialize V1 transaction");
        let ui_error: UiTransactionError =
            solana_sdk::transaction::TransactionError::AccountInUse.into();
        let transaction = EncodedConfirmedTransactionWithStatusMeta {
            slot: 42,
            transaction: EncodedTransactionWithStatusMeta {
                transaction: EncodedTransaction::Binary(
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                    TransactionBinaryEncoding::Base64,
                ),
                meta: Some(UiTransactionStatusMeta {
                    err: Some(ui_error.clone()),
                    status: Err(ui_error),
                    fee: 9_321,
                    pre_balances: vec![],
                    post_balances: vec![],
                    inner_instructions: OptionSerializer::None,
                    log_messages: OptionSerializer::None,
                    pre_token_balances: OptionSerializer::None,
                    post_token_balances: OptionSerializer::None,
                    rewards: OptionSerializer::None,
                    loaded_addresses: OptionSerializer::None,
                    return_data: OptionSerializer::None,
                    compute_units_consumed: OptionSerializer::Some(123_456),
                    cost_units: OptionSerializer::None,
                }),
                version: None,
            },
            block_time: None,
            transaction_index: None,
        };

        let (meta, _) = convert_rpc_to_grpc(&transaction).expect("convert failed RPC meta");
        let decoded_error: solana_sdk::transaction::TransactionError =
            wincode::deserialize(&meta.err.expect("failed status").err)
                .expect("deserialize transaction error");
        assert_eq!(decoded_error, solana_sdk::transaction::TransactionError::AccountInUse);

        let cost = parse_rpc_transaction_cost(&transaction).expect("parse failed RPC cost");
        assert!(!cost.tip_payments_confirmed);
        assert_eq!(cost.tip_lamports, 0);
        assert!(cost.tip_payments.is_empty());
        assert_eq!(cost.total_fee_and_tip_lamports, Some(9_321));
    }

    #[test]
    fn recognizes_every_sol_trade_sdk_provider() {
        assert_eq!(SWQOS_TIP_ACCOUNT_GROUPS.len(), 17);
        for group in SWQOS_TIP_ACCOUNT_GROUPS {
            assert!(!group.accounts.is_empty());
            assert_eq!(find_tip_provider(&group.accounts[0]), Some(group.provider));
        }
    }

    #[test]
    fn provider_tip_accounts_are_unique() {
        for (group_index, group) in SWQOS_TIP_ACCOUNT_GROUPS.iter().enumerate() {
            for (account_index, account) in group.accounts.iter().enumerate() {
                for candidate_group in &SWQOS_TIP_ACCOUNT_GROUPS[group_index..] {
                    let start = if candidate_group.provider == group.provider {
                        account_index + 1
                    } else {
                        0
                    };
                    assert!(
                        !candidate_group.accounts[start..].contains(account),
                        "duplicate tip account {account}"
                    );
                }
            }
        }
    }

    #[test]
    fn priority_fee_rounds_up_and_saturates() {
        assert_eq!(priority_fee_lamports(1, 1), 1);
        assert_eq!(priority_fee_lamports(200_000, 5_000), 1_000);
        assert_eq!(priority_fee_lamports(u32::MAX, u64::MAX), u64::MAX);
    }
}
