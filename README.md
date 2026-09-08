# Pump.fun Sniper Bot — Solana Memecoin Sniper (Rust + Jito Shredstream)

**Live Pump.fun sniper** for Solana. Detects new token creates from **Jito Shredstream**, buys in the same slot window, and exits on a delayed sell. Built in **Rust** with **Pump.fun**, **PumpSwap**, **Orca Whirlpool**, and **Jito bundle** execution — not a Node toy, not a log-subscribe bot.

**Contact / custom builds:** [t.me/dexoryn](https://t.me/dexoryn)

[![Solana](https://img.shields.io/badge/Solana-2.2.1-14F195?logo=solana&logoColor=white)](https://solana.com)
[![Rust](https://img.shields.io/badge/Rust-1.85.1-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Pump.fun](https://img.shields.io/badge/Pump.fun-Sniper-00FF88)](https://pump.fun)
[![Jito](https://img.shields.io/badge/Jito-Shredstream-FF6B00)](https://www.jito.wtf)
[![Telegram](https://img.shields.io/badge/Telegram-dexoryn-26A5E4?logo=telegram&logoColor=white)](https://t.me/dexoryn)

---

## Why this sniper

Most public “Pump.fun snipers” wait on RPC logs or Helius webhooks. That is already late.

This bot ingests **Jito Shredstream entries** over gRPC, deserializes shreds, and matches Pump.fun `create` instruction discriminators before the market is crowded. Buys go out with compute-budget + tip accounts (Astralane / Jito-style bribes). A sell is cached and fired a few slots later.

If you need a **private sniper**, copy-trade, or a custom shredstream cluster (Cherry / Teraswitch / Latitude), message **[t.me/dexoryn](https://t.me/dexoryn)**.

## Features

- **Pump.fun create sniper** — parse mint, creator, amounts, and blockhash from the create tx
- **Jito Shredstream** — `ShredstreamProxy` subscribe, not Yellowstone-only logs
- **Pump.fun SDK** — buy / sell instruction builder + versioned transactions
- **PumpSwap SDK** — AMM pool math, slippage, buy/sell execution
- **Orca Whirlpool** — state decode, tick arrays, Raydium / Kamino helpers
- **Jito searcher + SWQoS** — protobuf crate, bundle client, tip accounts
- **Yellowstone gRPC** — token-balance stream manager
- **Copy-trade listener** — follow wallets off the same shredstream pipe
- **Pinned stack** — Rust 1.85.1, Solana SDK 2.2.1, thin LTO release builds

## Stack

| Layer | Tech |
| --- | --- |
| Language | Rust 1.85.1 (`rustfmt` + `clippy`) |
| Chain | Solana 2.2.1 |
| Ingest | Jito Shredstream gRPC |
| Venues | Pump.fun · PumpSwap · Orca Whirlpool |
| Landing | Jito bundles / SWQoS + bribe tips |
| Streams | Yellowstone gRPC |
| Runtime | Tokio, moka cache, rustls |

## How it works

```text
Shredstream proxy  →  deserialize entries  →  scan Pump.fun program
        ↓
   create discriminator match
        ↓
   parse mint / creator / buy amounts / blockhash
        ↓
   PumpFunSDK.buy  (+ compute budget, ATA, tip)
        ↓
   cache FutureSell  →  sell N slots later
```

1. Connect to `SHREDSTREAM_URL` (Cherry, Teraswitch, or Latitude).
2. Decode `solana_entry::Entry` batches from each slot.
3. If an instruction hits the Pump.fun program and the create discriminator, parse the tx.
4. Submit a buy with `SOL_BUY_AMOUNT` and `BRIBE_FEE`.
5. Queue an automatic sell after a short slot delay.

## Requirements

- Rust **1.85.1** (see `rust-toolchain.toml`)
- A funded Solana wallet
- Shredstream endpoint (not public RPC)
- Optional: Jito block engine / SWQoS key for bundles

## Configuration

Copy `.example.env` → `.env` (gitignored — never commit keys):

```env
SOL_BUY_AMOUNT=0.001
BRIBE_FEE=0.001
SHREDSTREAM_URL=http://YOUR_SHREDSTREAM_HOST:9999
PVT_KEY=BASE58_PVT_KEY_HERE
```

| Variable | Description |
| --- | --- |
| `SOL_BUY_AMOUNT` | SOL size per Pump.fun buy |
| `BRIBE_FEE` | Landing tip (Jito / bribe) in SOL |
| `SHREDSTREAM_URL` | Jito shredstream proxy |
| `PVT_KEY` | Wallet secret in base58 |

## Build and run

```bash
cargo build --release
cargo run --release
```

Use `--release`. Debug builds are too slow for slot-race sniping.

## Repository layout

```text
src/main.rs              Pump.fun create sniper entrypoint
src/copy.rs              Copy-trade shredstream listener
src/grpc.rs              Yellowstone gRPC balance streams
src/types.rs             Shared buy/sell amounts and tx parsers
src/order_builder.rs     DEX-agnostic order routing
src/pumpfun/             Pump.fun constants, ix builder, SDK
src/pumpswap/            PumpSwap pool math, ix builder, SDK
src/whirlpool/           Orca state, ticks, Raydium/Kamino utils
jito_protos/             Jito protobuf + shredstream definitions
jito_sc/                 Searcher client + SWQoS gRPC
```

## Hire / custom sniper

This is production-style infrastructure, not a tutorial clone.

**Telegram: [t.me/dexoryn](https://t.me/dexoryn)**

- Private Pump.fun / PumpSwap snipers
- Shredstream + Jito bundle setups
- Copy-trade and multi-wallet execution
- Whirlpool / Raydium / Kamino routing
- Colocation (Cherry, Teraswitch, Latitude)

GitHub: [github.com/dexoryn](https://github.com/dexoryn)

## Disclaimer

Trading memecoins can result in total loss of funds. You own your keys, RPC/Shredstream costs, tips, and legal compliance. This repo is provided as-is. No profit is guaranteed.

Keep `.env` local. Rotate any key that has ever been pasted into chat, screenshots, or git history.
