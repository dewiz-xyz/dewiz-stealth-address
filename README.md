# dewiz-stealth-address

ERC-5564 stealth address implementation for EVM-compatible blockchains (Scheme ID 1: secp256k1 + view tags), written in Rust.

The library provides the full stealth address lifecycle: key generation, stealth address derivation, announcement scanning, private key recovery, and wallet creation for spending from stealth addresses.

## Main use case

Imagine Alice wants to send 100 USDS (sky stablecoin) to Bob in Ethereum blockchain. But she does not 
want to somebody else that receipient address (EOA) will be controlled by Bob. Even Bob does not know 
previoulsly which address (EOA) he will be able to get and moviment the funds (tokens).

This cryptographic strategy will be based on the idea that using Bob's Public Key of his EOA (address)
Alice will create a new key where only Bob will be able to decrypt it and moveiment the funds. Also,
based on this new private key, Alice will be able to calculate the Ethereum address where she will 
put as `to` parameter in her transfer transaction.

## Tech References

- [Liminal Cash - Stealth Address Thread](https://x.com/liminalcash/status/2015103091860033649)
- [SwapEscrow Drex - Cryptography](https://github.com/eybrativosdigitais/zapp-swapescrow-drex?tab=readme-ov-file#como-funciona-a-criptografia-no-swapescrow)
- [ERC-5564: Stealth Addresses](https://eips.ethereum.org/EIPS/eip-5564)
- [ERC-6538: Stealth Meta-Address Registry](https://eips.ethereum.org/EIPS/eip-6538)
- [ScopeLift - Stealth Address ERC Contracts](https://github.com/ScopeLift/stealth-address-erc-contracts)

### Liminal Cash Theory - Text based

The Visibility Problem
Every single on-chain transaction is permanently public. When you use a single wallet address for all financial activity, you create a complete financial profile visible to anyone:
asciidoc
Your Safe Wallet: 0xYourAddress
       │
       ├── Salary: $5,000/month from 0xEmployer
       ├── Rent: $2,000/month to 0xLandlord
       ├── Card spend: $47.32 at OnlyFans
       ├── DeFi: $10,000 in UwU Lend
       ├── Stock: 50 FISV dShares
       └── Offramp: $3,000 to 0xExGirlfriend

Anyone with a block explorer sees:
  ✓ Your income (amount, frequency, employer)
  ✓ Your expenses (rent, subscriptions, purchases)
  ✓ Your investments (DeFi positions, stock holdings)
  ✓ Your net worth (sum of all holdings)
  ✓ Your financial relationships (who pays you, who you pay)
The Chain Analysis Industry
Chain analysis firms like Chainalysis and Elliptic have built sophisticated 1984-grade surveillance infrastructure that processes blockchain data at scale. Chainalysis alone has clustered over 1 billion addresses across more than 55,000 services, wallets, and protocols (source).
Their systems use hundreds of clustering heuristics combined with machine learning to link addresses to real-world identities. In court proceedings, Chainalysis has claimed accuracy rates of 99.9146% for address attribution (Sterlingov case), though independent experts have disputed these claims - one CipherTrace expert describes certain behavioural clustering heuristics as "reckless" with accuracy discrepancies of roughly 64%. 
Regardless of exact accuracy, the threat model is clear: any address used more than once creates a persistent identity that can be linked, analysed and eventually attributed. 

Clustering Heuristics

Blockchain analytics rely on deterministic and probabilistic heuristics to group addresses controlled by the same entity (BACH Tool Paper). 
Co-Spend Heuristic (UTXO chains like Bitcoin): If address A and B are inputs to the same transaction, they are likely controlled by the same entity. 
Deposit Address Heuristic (Account-based chains like Ethereum): Track flow from deposit addresses to consolidation wallets. If addresses X, Y, Z all send to the same hot wallet, they are likely deposit addresses for the same service. 
Event-Based Heuristics (Smart Contract chains): Monitor factory contracts for deployment patterns. If the same deployer creates multiple contracts, those contracts are linked to the same entity.
Gas Price Fingerprinting: Custom gas prices (non-standard Gwei multiples) act as fingerprints. If two transactions use gas price 12.123456789 Gwei, they likely originated from the same user. 

The Dual-Use Problem

Surveillance capabilities exist to catch bad actors. Blockchain traceability has enabled some of the most significant financial crime recoveries in history - from seizing $3.6 billion in stolen Bitcoin to linking a $625 million hack to Lazarus Group within weeks.

The technology is genuinely effective at catching criminals.

The problem is: it catches everyone else too. Traditional finance maintains a spectrum of privacy, from untraceable cash transfers to court-ordered subpoenas (investigations must justify access to specific accounts for specific reasons). 
Blockchain inverts this model entirely. This creates a risk of concrete harms: 
Wealth exposure creates physical risk. Public balances make users targets for kidnapping, extortion and physical coercion.
Transaction histories reveal your personal data. Your political affiliations (donations), sexuality (dating app subscriptions and adult content purchases), vices (gambling sites), and even medical conditions (pharmacy payments) are accessible to anyone that knows your address. God forbid you bought a .eth in your name.

Permanence eliminates redemption. A single embarrassing transaction - or a false positive from a clustering algorithm - becomes a permanent part of your financial identity. There is no right to be forgotten. 

Privacy Goal

The goal: Break these links so that even with full blockchain visibility, an observer cannot aggregate a user's complete financial picture.
Specifically, we want to achieve:

Unlinkability: Given two stealth addresses A and B belonging to the same user, an observer cannot determine that A and B are related.
Untraceability: Given a payment from Alice to Bob's stealth address, an observer cannot identify Bob as the recipient.
Balance Hiding: An observer cannot determine the total balance held across a user's stealth addresses.
Pattern Hiding: An observer cannot build behavioural profiles from fragmented transaction histories.

This requires breaking the clustering heuristics at every level - deployment, funding, interaction and withdrawal - which is exactly what the Stealth Safe architecture achieves through ERC-5564 stealth address, counterfactual Safe deployment and ERC-4337 paymaster gas sponsorship. 

Receiving Privately: Bob's Perspective

ERC-5564 stealth addresses solve the recipient privacy problem. When someone sends you money, observers cannot identify you as the recipient. How does this work?

The Mathematics of Stealth Addresses

Bob wants to receive birthday money, but doesn't want anyone to know how much he's getting or who's sending it.
Setup: Bob publishes his stealth meta-address
Bob doesn't give out his public wallet address. Instead, he publishes instructions for how to create a wallet that only he can open.

julia
CURVE PARAMETERS (secp256k1)
════════════════════════════
G = generator point
n = curve order

Spending keypair:
  k ← [1, n-1]        K = k·G

Viewing keypair:
  v ← [1, n-1]        V = v·G

Meta-address:
  st:eth:0x || compress(K) || compress(V)
  └─prefix─┘   └─33 bytes──┘ └─33 bytes──┘
Send: Alice generates stealth address for Bob
Alice doesn't send money to Bob's address (he doesn't have one!). Instead, she follows Bob's instructions: Alice adds her own random hash, creates a new wallet for Bob, and adds funds.

julia

1. Generate ephemeral keypair:
   r ← [1, n-1]
   R = r·G                    (published in announcement)

2. ECDH shared secret:
   S = r·V                    (Alice computes)
     = r·v·G                  (equivalent form)

3. Hash to scalar:
   s = keccak256(0x04 || Sₓ || Sᵧ)
       └──────SEC1 uncompressed──────┘

4. Derive stealth public key:
   P = K + s·G

5. Compute Ethereum address:
   addr = keccak256(Pₓ || Pᵧ)[12:32]
          └───last 20 bytes───┘

6. View tag (optimization):
   tag = s[0]                 (first byte of hash)
Receive: Bob detects and claims payment
That's it. Every time someone pays Bob, a fresh new wallet is created. Even if you watch Bob receive 100 payments, you see 100 unconnected wallets - not "Bob's profile with 100 transfers."

julia

1. Scan announcements for matching view tag:
   For each (R, tag, addr) in announcements:
     S' = v·R                 (Bob computes using viewing key)
     s' = keccak256(0x04 || S'ₓ || S'ᵧ)
     if s'[0] ≠ tag: skip     (99.6% filtered here)

2. Verify full address:
   P' = K + s'·G
   addr' = keccak256(P'ₓ || P'ᵧ)[12:32]
   if addr' = addr: payment found!

3. Derive stealth private key:
   p = k + s' mod n

   Verification: p·G = k·G + s'·G = K + s'·G = P ✓

This flow breaks the fundamental link between sender and recipient. Alice can pay Bob without knowing his wallet address. Bob can receive funds without revealing his identity. An observer monitoring the blockchain sees only disconnected, single-use addresses with no transaction history and no cryptographic link to any identity. Each payment exists in isolation - 100 payments to Bob appear as 100 unrelated wallets, not a financial profile.

The Technology of Stealth Addresses

ERC-5564 defines a schema-agnostic protocol for stealth addresses. The standard introduces:
Scheme ID: A 256-bit identifier for the cryptographic scheme. Scheme ID 1 is the SECP256k1 with view tags.
Stealth Meta-Address: The recipient's public information needed to derive stealth addresses.

Announcer Contract: Singleton contract at  0x55649E01B5Df198D18D95b5cc5051630cfD45564 that emits payment announcements

View Tags: 1-byte optimization for ~6x faster announcement scanning

Does This Solve Onchain Privacy?

All an observer sees onchain is that Alice sent funds to a stealth address and the announcement event. The observer cannot link the stealth address to any known entity, cannot link to Bob's other stealth addresses and cannot determine if it is even a stealth address as it looks like any other EOA. 

On the backend, via the viewing key, Bob can scan announcements, detect payments, compute stealth address from the ephemeral key, index payments and track balances. 

On the frontend, Bob can derive the stealth private key, sign transactions and (privately?) spend funds.
So recipient privacy is solved. But there's a catch that breaks everything in practice. That, is the funding problem. 

## On-chain Contracts (ERC-5564 & ERC-6538)

Two singleton contracts are deployed at the same address on all supported chains via CREATE2:

| Contract           | Address                                      | Purpose                                                   |
|--------------------|----------------------------------------------|-----------------------------------------------------------|
| ERC-5564 Announcer | `0x55649E01B5Df198D18D95b5cc5051630cfD45564` | Sender publishes `(R, tag, stealth_addr)` announcements   |
| ERC-6538 Registry  | `0x6538E6bf4B0eBd30A8Ea093027Ac2422ce5d6538` | Recipient registers their stealth meta-address            |

### Mainnets (7 chains)

| Chain        | Explorer (Announcer)                                                                                                  |
|--------------|-----------------------------------------------------------------------------------------------------------------------|
| Ethereum     | [Etherscan](https://etherscan.io/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                                  |
| Arbitrum     | [Arbiscan](https://arbiscan.io/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                                    |
| Base         | [Basescan](https://basescan.org/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                                   |
| Gnosis Chain | [Gnosisscan](https://gnosisscan.io/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                                |
| Optimism     | [Optimistic Etherscan](https://optimistic.etherscan.io/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)            |
| Polygon      | [Polygonscan](https://polygonscan.com/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                             |
| Scroll       | [Scrollscan](https://scrollscan.com/address/0x55649E01B5Df198D18D95b5cc5051630cfD45564)                               |

### Testnets (5 chains)

Sepolia, Holesky, Arbitrum Sepolia, Base Sepolia, Optimism Sepolia

### On-chain flow

1. **Bob** calls `ERC6538Registry.registerKeys()` to publish his stealth meta-address
2. **Alice** reads Bob's meta-address from the registry, generates a stealth address using this library, sends tokens to it, and calls `ERC5564Announcer.announce()` with `(R, tag, stealth_addr)`
3. **Bob** scans announcement events, detects payments, and derives the stealth private key to move the funds

Source: [ScopeLift/stealth-address-erc-contracts](https://github.com/ScopeLift/stealth-address-erc-contracts)

### Contract interfaces (Solidity)

```solidity
// SPDX-License-Identifier: MIT

// ERC-5564 Announcer — deployed at 0x55649E01B5Df198D18D95b5cc5051630cfD45564
interface IERC5564Announcer {
    event Announcement(
        uint256 indexed schemeId,
        address indexed stealthAddress,
        address indexed caller,
        bytes ephemeralPubKey,
        bytes metadata
    );

    /// @param schemeId        Scheme identifier (1 = secp256k1 with view tags)
    /// @param stealthAddress  The derived stealth address
    /// @param ephemeralPubKey The sender's ephemeral public key R (33 bytes compressed)
    /// @param metadata        First byte = view tag, remainder is optional (token info, amount, etc.)
    function announce(
        uint256 schemeId,
        address stealthAddress,
        bytes memory ephemeralPubKey,
        bytes memory metadata
    ) external;
}

// ERC-6538 Registry — deployed at 0x6538E6bf4B0eBd30A8Ea093027Ac2422ce5d6538
interface IERC6538Registry {
    event StealthMetaAddressSet(
        address indexed registrant,
        uint256 indexed schemeId,
        bytes stealthMetaAddress
    );

    /// @param schemeId             Scheme identifier (1 = secp256k1 with view tags)
    /// @param stealthMetaAddress   The stealth meta-address bytes (compress(K) || compress(V) = 66 bytes)
    function registerKeys(uint256 schemeId, bytes calldata stealthMetaAddress) external;

    function stealthMetaAddressOf(
        address registrant,
        uint256 schemeId
    ) external view returns (bytes memory);
}
```

### Call examples (Solidity)

```solidity
// Bob registers his stealth meta-address
IERC6538Registry registry = IERC6538Registry(0x6538E6bf4B0eBd30A8Ea093027Ac2422ce5d6538);

// stealthMetaAddress = compress(K) || compress(V), 66 bytes
bytes memory stealthMetaAddress = abi.encodePacked(compressedSpendingPubKey, compressedViewingPubKey);
registry.registerKeys(1, stealthMetaAddress);

// Alice reads Bob's meta-address
bytes memory bobMeta = registry.stealthMetaAddressOf(bobAddress, 1);

// After generating stealth address off-chain, Alice announces the payment
IERC5564Announcer announcer = IERC5564Announcer(0x55649E01B5Df198D18D95b5cc5051630cfD45564);

// metadata: first byte = view tag, then token info
bytes memory metadata = abi.encodePacked(viewTag, bytes4(0xeeeeeeee), stealthAddr, amount);
announcer.announce(1, stealthAddr, ephemeralPubKey, metadata);
```

### Call examples (Rust with alloy 1.7.3)

```rust
use alloy::primitives::{address, Address, Bytes, U256};
use alloy::providers::ProviderBuilder;
use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IERC5564Announcer {
        function announce(
            uint256 schemeId,
            address stealthAddress,
            bytes memory ephemeralPubKey,
            bytes memory metadata
        ) external;
    }

    #[sol(rpc)]
    interface IERC6538Registry {
        function registerKeys(uint256 schemeId, bytes calldata stealthMetaAddress) external;
        function stealthMetaAddressOf(address registrant, uint256 schemeId)
            external view returns (bytes memory);
    }
}

const ANNOUNCER: Address = address!("55649E01B5Df198D18D95b5cc5051630cfD45564");
const REGISTRY: Address  = address!("6538E6bf4B0eBd30A8Ea093027Ac2422ce5d6538");
const SCHEME_ID: U256 = U256::from_limbs([1, 0, 0, 0]); // secp256k1 with view tags

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let provider = ProviderBuilder::new()
        .wallet(your_signer)
        .connect("https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY")
        .await?;

    // --- Bob registers his stealth meta-address ---
    let registry = IERC6538Registry::new(REGISTRY, &provider);

    // stealth_meta_address = compress(K) || compress(V), 66 bytes
    let stealth_meta_address: Bytes = [
        compressed_spending_pubkey.as_slice(),
        compressed_viewing_pubkey.as_slice(),
    ].concat().into();

    let tx = registry
        .registerKeys(SCHEME_ID, stealth_meta_address)
        .send()
        .await?
        .watch()
        .await?;
    println!("registerKeys tx: {tx}");

    // --- Alice reads Bob's meta-address ---
    let bob_meta = registry
        .stealthMetaAddressOf(bob_address, SCHEME_ID)
        .call()
        .await?;
    // bob_meta._0 contains the raw bytes — parse with this library's parse_meta_address()

    // --- Alice announces the payment ---
    let announcer = IERC5564Announcer::new(ANNOUNCER, &provider);

    // metadata: first byte = view tag, rest is optional
    let mut metadata = vec![view_tag];
    // optionally append token info per ERC-5564 spec

    let tx = announcer
        .announce(
            SCHEME_ID,
            stealth_address,
            Bytes::from(ephemeral_pubkey_compressed),
            Bytes::from(metadata),
        )
        .send()
        .await?
        .watch()
        .await?;
    println!("announce tx: {tx}");

    Ok(())
}
```

## Build & Test

```bash
cargo build                                                          # compile
cargo clippy                                                         # lint — must pass with zero warnings
RUSTFLAGS="-C target-cpu=native -C link-arg=-s" cargo test           # run all tests (unit + integration)
cargo run                                                            # run CLI demo (full Alice→Bob flow)
```

Integration tests in `tests/onchain.rs` require a `.env` file (see `env.example`) with `RPC_URL`, `PRIVATE_KEY_SENDER`, and `PRIVATE_KEY_RECEIVER`.

## Project Structure

| Path | Purpose |
|------|---------|
| `src/lib.rs` | Crate root, re-exports `stealth` module |
| `src/stealth.rs` | Core protocol: types (`StealthMetaAddress`, `StealthOutput`, `RecoveredKey`), public API, internal helpers, 26 unit tests |
| `tests/common/mod.rs` | Shared test infrastructure (`TestApp`, wallet setup, provider config) |
| `tests/common/smartcontract/abi.rs` | ERC-20 ABI binding via alloy `sol!` macro with `#[sol(rpc)]` |
| `tests/env.rs` | Environment / wallet address integration tests |
| `tests/keys.rs` | Key generation and stealth address flow tests |
| `tests/token.rs` | ERC-20 contract address validation tests |
| `tests/onchain.rs` | Full round-trip on-chain test: generate stealth address → send USDC & ETH → recover key → spend from stealth wallet |
| `Cargo.toml` | Dependencies and project metadata |

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `k256` | secp256k1 curve arithmetic (`ProjectivePoint`, `Scalar`, `NonZeroScalar`) |
| `sha3` | Keccak-256 hashing |
| `rand_core` | `OsRng` for cryptographic random scalar generation |
| `hex` | Hex encoding/decoding |
| `alloy` | Ethereum types (`Address`), wallet/signer (`EthereumWallet`, `PrivateKeySigner`), RPC provider, `sol!` macro |

## Public API

### Types

- **`StealthMetaAddress`** — spending + viewing keypairs; constructed via `generate_meta_address()`, `from_secp256k1_nonzeroscalar()`, or `from_private_key_string()`
- **`StealthOutput`** — result of `generate_stealth_address()`: ephemeral pubkey, stealth pubkey/address, view tag; has `.to_ethereum_address()` → `alloy::Address`
- **`RecoveredKey`** — result of `scan_and_recover()`: stealth private key + address; has `.to_ethereum_address()` and `.to_wallet()` → `EthereumWallet`

### Functions

- `generate_meta_address()` — generate a fresh stealth meta-address (random spending + viewing keys)
- `generate_stealth_address(K, V)` — derive a one-time stealth address from recipient's public keys
- `scan_and_recover(meta, R, tag, addr)` — scan an announcement and recover the stealth private key
- `verify(priv_key, pubkey)` — verify a recovered private key matches the expected stealth public key
- `parse_meta_address(str)` / `format_meta_address(meta)` — serialize/deserialize `st:eth:0x...` format
- `point_to_hex()`, `scalar_to_hex()`, `addr_to_hex()` — display helpers

## Protocol Encoding Rules

- Shared secret hash: `keccak256(0x04 || Sx || Sy)` — SEC1 uncompressed, all 65 bytes
- Ethereum address: `keccak256(Px || Py)[12:32]` — raw 64 bytes, skip `0x04` tag, take last 20
- Meta-address format: `st:eth:0x{compress(K)}{compress(V)}` — 33+33 bytes hex-encoded
- Scalar from hash: `Reduce<U256>::reduce_bytes` for mod n reduction

## Conventions

- All public API lives in `src/stealth.rs`
- No `unwrap()` in library code — return `Result` or `Option`
- `cargo clippy` must pass with zero warnings before any commit
- Unit tests go in `#[cfg(test)] mod tests` inside `stealth.rs`
- Integration tests use shared `tests/common/` module pattern (no `#[path]` macros needed)

## Technology

Built with Rust (edition 2021) using secp256k1 elliptic curve cryptography. On-chain interactions use the [alloy](https://github.com/alloy-rs/alloy) Ethereum toolkit for type-safe RPC calls, contract bindings, and wallet management.

