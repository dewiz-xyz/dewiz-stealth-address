# Your USDS Payments Are Public. We're Fixing That.

## How Dewiz Stealth Addresses Bring Private E-Commerce to Sky's Stablecoin via the x402 Payment Standard

---

USDS is the third-largest stablecoin in the world. With over $9 billion in circulating supply, 74% growth last year, and the Sky Frontier Foundation projecting a path to $20 billion in 2026, USDS is becoming the go-to yield-generating stablecoin for DeFi participants and institutional capital alike.

But here's a problem nobody is talking about enough: **every USDS payment you make is permanently visible to the entire world.**

When you pay for an API call, buy a product from an e-commerce merchant, or tip a creator — anyone with a block explorer can see your wallet, the amount, the recipient, and the timestamp. String a few transactions together and you've got a complete financial profile: your income, your spending habits, your business relationships.

Chain analysis firms have clustered over a billion addresses. Their heuristics link wallets through co-spend patterns, gas price fingerprints, and deposit address flows. The result? Using a single wallet for USDS payments is like publishing your bank statements on Twitter.

**At Dewiz, we build infrastructure for the Sky Ecosystem.** We're the team that helps craft and deploy Sky Protocol's executive spells, and we've been deep in the protocol's smart contract architecture since the MakerDAO days. Now, we're bringing that same engineering rigor to a problem that matters to every USDS holder: **payment privacy.**

---

## The Missing Piece: x402 + Stealth Addresses

Two technologies are converging that make private stablecoin e-commerce a reality:

**x402** is the new HTTP payment standard (backed by Coinbase, Cloudflare, Anthropic, Circle, and others) that embeds stablecoin payments directly into web requests. A server returns HTTP 402 "Payment Required," your wallet signs a payment authorization, and the transaction settles on-chain in milliseconds. No API keys. No subscriptions. No checkout forms. Just HTTP + stablecoins.

It's elegant — but it's also fully transparent. Every x402 payment links your wallet to the merchant's address on a public ledger.

**ERC-5564 Stealth Addresses** solve this. Instead of paying to a merchant's known wallet, your payment goes to a fresh, one-time address that only the merchant can unlock. An observer watching the blockchain sees funds going to what looks like a random new wallet — not "the API you've been using three times a day."

We built **dewiz-stealth-address** — a Rust implementation of the complete ERC-5564 stealth address lifecycle — and we're integrating it with x402 to bring private USDS payments to the internet.

---

## How It Works (For the Technically Curious)

The cryptography is based on a simple but powerful idea: **Elliptic Curve Diffie-Hellman (ECDH) shared secrets.**

**Setup:** a merchant publishes a stealth meta-address — essentially two public keys (spending + viewing) registered on-chain via the ERC-6538 Registry (deployed at `0x6538...6538` across 7 mainnets).

**Payment:** when you make an x402 payment with USDS, instead of sending to a static address, the system generates a one-time stealth address from the merchant's public keys combined with a random ephemeral keypair. The math guarantees that **only the merchant** can derive the private key for that address.

**Discovery:** the merchant's scanner daemon watches ERC-5564 Announcer events (singleton at [`0x55649E01B5Df198D18D95b5cc5051630cfD45564`](https://etherscan.io/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)), uses their viewing key to filter events (view tags eliminate 99.6% of non-matching events in one byte comparison), and recovers the stealth private key to access the funds.

**Result:** 100 different payments to the same merchant appear as 100 different payments to unconnected wallets. No clustering. No profiling. No financial surveillance.

The ERC-5564 Announcer and ERC-6538 Registry contracts used for stealth addresses are already deployed on Ethereum, Arbitrum, Base, Gnosis Chain, Optimism, Polygon, and Scroll. No new stealth-address contract deployment is needed.

---

## Why This Matters for USDS Holders

**If you're a trader:** your on-chain footprint is your competitive vulnerability. Every USDS payment reveals information — which services you use, which data feeds you buy, how your strategies evolve. Stealth addresses break the information leakage chain.

**If you're a merchant or an API provider:** accepting USDS via x402 is great for cash flow. But accumulating all payments to a single wallet exposes your revenue, client count, and business relationships. With stealth addresses, your income is fragmented across unlinkable wallets that only you can find.

**If you're an AI agent operator:** the x402 standard was designed for autonomous agent payments. But an AI agent making hundreds of API calls from one wallet creates the richest behavioral profile chain analysts have ever seen. Stealth addresses give your agents the same privacy you'd expect from a VPN — but for financial transactions.

**If you care about the Sky Ecosystem:** adoption of USDS at $20B+ scale means USDS will be used for everything — payroll, subscriptions, e-commerce, micropayments. Without privacy tooling, USDS becomes a surveillance asset by default. Stealth address integration makes USDS viable for the full spectrum of real-world commerce.

---

## Privacy ≠ Anonymity: the Compliance Story

Let's be clear: this isn't Tornado Cash. Dewiz builds compliance-first infrastructure.

The viewing key architecture of ERC-5564 enables **selective disclosure**. A merchant can share their viewing key with auditors or regulators, giving authorized parties full visibility into all incoming payments — without making that information publicly available on-chain.

This aligns with Sky Protocol's own direction. USDS includes a freeze function for regulatory compliance. Stealth addresses add a privacy layer on top of that compliance foundation — protecting legitimate users from surveillance while maintaining the ability to cooperate with authorized oversight.

---

## The Tech Stack

Our implementation is production-grade Rust, built with the `alloy` Ethereum toolkit:

- **Core library:** `dewiz-stealth-address` — full ERC-5564 lifecycle (Scheme ID 1: secp256k1 + one-byte view tags used to quickly filter announcements);
- **Cryptographic foundation:** `k256` for curve arithmetic, `sha3` for Keccak-256;
- **Integration ready:** generates `alloy::Address` and `EthereumWallet` directly from recovered stealth keys;
- **Battle-tested:** full round-trip on-chain integration tests (generate stealth address → send tokens → recover key → spend from stealth wallet);
- **Zero unwrap:** all library code returns `Result` or `Option` — no panics in production.

The code is open source: [github.com/dewiz-xyz/dewiz-stealth-address](https://github.com/dewiz-xyz/dewiz-stealth-address)

---

## What's Next

We're building the x402-stealth-middleware — a Rust crate with Node.js/Python FFI bindings that plugs directly into any x402 server to replace static payment addresses with per-request stealth addresses. The middleware handles stealth address generation, ERC-5564 announcement emission, and integrates with ERC-4337 paymasters to solve the gas funding problem for stealth wallets.

Target networks include Base (primary — home of x402), Ethereum, Arbitrum, and Optimism.

The goal is simple: **make every USDS payment over x402 private by default.**

---

## Supporting the Sky Ecosystem

Dewiz has been a technical pillar of the Sky/MakerDAO ecosystem — from executive spells to smart contract infrastructure. We believe USDS will be the stablecoin that the internet economy runs on. But to get there, it needs privacy tooling that matches its scale ambitions.

Stealth addresses + x402 = private internet-native USDS payments. That's the future we're building.

---

*Dewiz — Infrastructure for the Sky Ecosystem*

🔗 [dewiz-stealth-address on GitHub](https://github.com/dewiz-xyz/dewiz-stealth-address)
🔗 [x402 Protocol](https://www.x402.org/)
🔗 [ERC-5564 Specification](https://eips.ethereum.org/EIPS/eip-5564)
🔗 [Sky Protocol](https://sky.money/)

---

*This article is for informational purposes. It does not constitute financial advice. Stealth address technology provides transaction-level privacy and does not guarantee complete anonymity. Always comply with local regulations when using stablecoins.*
