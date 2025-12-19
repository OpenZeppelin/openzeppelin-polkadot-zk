# Polkadot Confidential Assets Framework

**WARNING: This is beta software that has NOT been audited and is NOT ready for production. Use at your own risk!**

Rust libraries for verifying computation executed on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Performance Benchmarks](#performance-benchmarks)
- [Quick Start](#quick-start)
- [Usage Examples](#usage-examples)
- [Cross-Chain Transfers](#cross-chain-transfers)
- [PolkaVM Precompile](#polkavm-precompile)
- [Testing](#testing)
- [References](#references)
- [Security Policy](#security-policy)

## Overview

This framework implements **ERC-7984 Confidential Contracts** for Polkadot, enabling:

- **Private Balances**: All balances stored as Pedersen commitments (32 bytes)
- **ZK-Verified Transfers**: Zero-knowledge proofs guarantee correctness without revealing amounts
- **Cross-Chain Privacy**: Confidential transfers between parachains via HRMP
- **Two-Phase Transfers**: Decoupled sender/receiver for async settlement

### Design Principles

1. **Confidentiality, not anonymity**: Account addresses are public, only amounts are private
2. **Separate paths**: Confidential and public assets never mix
3. **On-chain verification**: All ZK proofs verified on-chain for trustless operation

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                        User Application                         │
├─────────────────────────────────────────────────────────────────┤
│  zkhe-prover (off-chain)              │  Client SDK             │
│  - prove_sender_transfer()            │  - Key management       │
│  - prove_receiver_accept()            │  - Balance tracking     │
│  - prove_mint() / prove_burn()        │  - UTXO selection       │
├───────────────────────────────────────┴─────────────────────────┤
│                       Extrinsic Submission                      │
├─────────────────────────────────────────────────────────────────┤
│                   On-Chain (Parachain Runtime)                  │
│  ┌─────────────────────┐  ┌─────────────────────┐               │
│  │ pallet-confidential │  │ pallet-confidential │               │
│  │       -assets       │  │       -bridge       │               │
│  │   (IERC7984 API)    │  │    (Cross-chain)    │               │
│  └──────────┬──────────┘  └──────────┬──────────┘               │
│             │                        │                          │
│  ┌──────────▼──────────┐  ┌──────────▼──────────┐               │
│  │    pallet-zkhe      │  │ pallet-confidential │               │
│  │ (Backend + Storage) │  │       -escrow       │               │
│  └──────────┬──────────┘  └─────────────────────┘               │
│             │                                                   │
│  ┌──────────▼──────────┐                                        │
│  │   zkhe-verifier     │  ← no_std, runs on-chain               │
│  │  (ZK Proof Verify)  │                                        │
│  └─────────────────────┘                                        │
├─────────────────────────────────────────────────────────────────┤
│  pallet-operators (delegation) │ pallet-acl (pause/limits)      │
└─────────────────────────────────────────────────────────────────┘
```

### Pallets

| Pallet | Purpose |
|--------|---------|
| `pallet-confidential-assets` | User-facing API following IERC7984 standard |
| `pallet-zkhe` | ZK backend storing encrypted balances + verification |
| `pallet-confidential-bridge` | Cross-chain confidential transfers via HRMP |
| `pallet-confidential-escrow` | Escrow adapter for atomic swaps |
| `pallet-operators` | Operator permissions for delegated transfers |
| `pallet-acl` | Access control, pause gates, per-tx limits |

### Prover + Verifier

| Crate | Environment | Purpose |
|-------|-------------|---------|
| `zkhe-prover` | Off-chain (std) | Generate ZK proofs for transfers |
| `zkhe-verifier` | On-chain (no_std) | Verify ZK proofs in runtime |

### Runtimes

| Runtime | Purpose |
|---------|---------|
| `runtimes/polkavm` | PolkaVM-based runtime with pallet-revive for smart contract execution |

## Performance Benchmarks

Benchmarks run on Apple M1 Pro (10 cores), native execution.

### TL;DR

- **~37 complete confidential transfers per second** (sender + receiver proofs)
- **~100 TPS** if only counting sender-initiated transfers per block
- **Privacy costs ~90% throughput** compared to standard Polkadot transfers
- **Bottleneck**: Bulletproof range proofs (~2-4ms each)

### Understanding Two-Phase Transfers

A confidential transfer requires **two on-chain transactions**:

1. **Sender phase** (`verify_transfer_sent`): Alice submits proof, creates encrypted UTXO for Bob
2. **Receiver phase** (`verify_transfer_received`): Bob claims the UTXO, moves to available balance

These can happen in the same block or different blocks. The TPS depends on which you're measuring.

### Verification Times

| Operation | Time | Bottleneck |
|-----------|------|------------|
| Sender proof | 2.52ms | 1 Bulletproof range proof (~2.2ms) |
| Receiver proof | 4.39ms | 2 Bulletproof range proofs (~4.4ms) |
| **Complete transfer** | **~7ms** | 3 range proofs total |

The Σ-proof algebra (ElGamal + Pedersen) is fast (~0.3ms). **Bulletproofs dominate verification cost.**

### Block Throughput

Polkadot parachains: 6s blocks, ~1.5s compute budget for normal extrinsics.

| Scenario | Txs/Block | TPS | Use Case |
|----------|-----------|-----|----------|
| Sender-only | 604 | ~100 | Measuring send throughput |
| Receiver-only | 351 | ~58 | Measuring claim throughput |
| **Complete (both)** | **223** | **~37** | **Realistic end-to-end** |

**Cost drift < 2%**: The 600th transaction in a block costs the same as the 1st. No degradation as blocks fill.

### Ecosystem Comparison

| Chain | TPS | Notes |
|-------|-----|-------|
| Polkadot (standard transfers) | ~1,000 | Plain balance transfers |
| Kusama (stress test) | ~10,547 | 2024 "Spammening" peak |
| **Confidential transfers** | **~37-100** | This framework |

**Privacy has a cost**: ~10% of standard throughput. This is expected - you're verifying zero-knowledge proofs on every transaction.

### Running Benchmarks

```bash
# Run comprehensive TPS benchmark
cargo run -p confidential-benchmarks --release

# Run criterion benchmarks
cargo bench -p confidential-benchmarks

# Run pallet benchmarks (WASM)
frame-omni-bencher v1 benchmark pallet \
  --runtime target/release/wbuild/confidential-asset-hub/confidential_asset_hub.compact.compressed.wasm \
  --pallet pallet_zkhe \
  --extrinsic '*' \
  --steps 50 --repeat 20
```

## Quick Start

### Prerequisites

```bash
# Rust toolchain
rustup default stable
rustup target add wasm32-unknown-unknown

# For WASM builds (macOS)
brew install llvm
export CC_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/clang"
export AR_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/llvm-ar"
```

### Build

```bash
# Build all crates
cargo build --release

# Build runtime WASM
cargo build --release -p confidential-asset-hub

# Run tests
cargo test --workspace
```

## Usage Examples

### 1. Setting Up a Confidential Account

```rust
use zkhe_prover::{SenderInput, prove_sender_transfer};
use curve25519_dalek::ristretto::RistrettoPoint;
use rand::rngs::OsRng;

// Generate ElGamal keypair
fn generate_keypair() -> (Scalar, RistrettoPoint) {
    let sk = Scalar::random(&mut OsRng);
    let pk = sk * RISTRETTO_BASEPOINT_POINT;
    (sk, pk)
}

// Register public key on-chain
// Call: ConfidentialAssets::set_public_key(origin, pk_bytes)
```

### 2. Making a Confidential Transfer (Two-Phase)

**Phase 1: Sender creates transfer proof**

```rust
use zkhe_prover::{SenderInput, prove_sender_transfer};

let sender_input = SenderInput {
    asset_id: asset_id.encode(),
    network_id: [0u8; 32], // Your network ID

    sender_pk: alice_pk,
    receiver_pk: bob_pk,

    // Alice's current balance commitment and opening
    from_old_c: alice_balance_commitment,
    from_old_opening: (alice_balance_value, alice_balance_blind),

    // Bob's current pending commitment (empty if first transfer)
    to_old_c: bob_pending_commitment,

    // Amount to transfer
    delta_value: 1000,

    // Random seed for proof generation
    rng_seed: [0u8; 32], // Use secure random in production!

    fee_c: None,
};

let output = prove_sender_transfer(&sender_input)?;

// Submit to chain:
// ConfidentialAssets::confidential_transfer(
//     origin,
//     asset_id,
//     bob_account,
//     output.delta_ct_bytes,      // Encrypted amount
//     output.sender_bundle_bytes, // Proof
// )
```

**Phase 2: Receiver claims pending deposit**

```rust
use zkhe_prover::{ReceiverAcceptInput, prove_receiver_accept};

let accept_input = ReceiverAcceptInput {
    asset_id: asset_id.encode(),
    network_id: [0u8; 32],

    receiver_pk: bob_pk,

    // Bob's current available and pending balances
    avail_old_c: bob_available_commitment,
    avail_old_opening: (bob_available_value, bob_available_blind),

    pending_old_c: bob_pending_commitment,
    pending_old_opening: (bob_pending_value, bob_pending_blind),

    // The transfer to claim (from sender's output)
    delta_comm: delta_commitment_from_sender,
    delta_value: 1000,
    delta_rho: delta_blind_from_decryption,
};

let output = prove_receiver_accept(&accept_input)?;

// Submit to chain:
// ConfidentialAssets::confidential_claim(
//     origin,
//     asset_id,
//     output.accept_envelope,
// )
```

### 3. Depositing (Public → Confidential)

```rust
// Shield public tokens into confidential balance
use zkhe_prover::{MintInput, prove_mint};

let mint_input = MintInput {
    asset_id: asset_id.encode(),
    network_id: [0u8; 32],

    to_pk: alice_pk,

    // Current pending balance (identity if empty)
    to_pending_old_c: alice_pending_commitment,
    to_pending_old_opening: (0, Scalar::ZERO),

    // Current total supply
    total_old_c: total_supply_commitment,
    total_old_opening: (total_supply_value, total_supply_blind),

    mint_value: 10000, // Amount to deposit
    rng_seed: [0u8; 32],
};

let output = prove_mint(&mint_input)?;

// Submit: ConfidentialAssets::deposit(origin, asset_id, amount, proof)
```

### 4. Withdrawing (Confidential → Public)

```rust
use zkhe_prover::{BurnInput, prove_burn};

let burn_input = BurnInput {
    asset_id: asset_id.encode(),
    network_id: [0u8; 32],

    from_pk: alice_pk,

    from_avail_old_c: alice_available_commitment,
    from_avail_old_opening: (alice_balance, alice_blind),

    total_old_c: total_supply_commitment,
    total_old_opening: (total_supply, total_blind),

    burn_value: 5000, // Amount to withdraw
    rng_seed: [0u8; 32],
};

let output = prove_burn(&burn_input)?;

// Submit: ConfidentialAssets::withdraw(origin, asset_id, encrypted_amount, proof)
```

## Cross-Chain Transfers

The framework supports confidential transfers between parachains via HRMP.

### Architecture

```text
ParaA (Sender)                    ParaB (Receiver)
┌─────────────┐                   ┌─────────────┐
│ 1. Lock     │                   │             │
│    funds in │    HRMP Message   │ 3. Mint     │
│    escrow   │ ─────────────────▶│    on dest  │
│             │                   │             │
│ 2. Send     │                   │ 4. Confirm  │
│    packet   │ ◀─────────────────│    success  │
│             │    XCM Response   │             │
│ 5. Burn     │                   │             │
│    escrow   │                   │             │
└─────────────┘                   └─────────────┘
```

### Cross-Chain Transfer Flow

```rust
// On source chain (ParaA):
ConfidentialBridge::send_confidential(
    dest_para_id,       // Destination parachain
    dest_account,       // Recipient on destination
    asset_id,
    encrypted_amount,
    lock_proof,         // Proves sender can lock the amount
    accept_envelope,    // Receiver's acceptance proof
)

// On destination chain (ParaB) - executed via XCM:
// Automatically mints confidential balance to recipient

// On source chain - after confirmation:
// Escrow is burned, transfer is finalized
```

## PolkaVM Precompile

The framework includes a Solidity-compatible precompile for PolkaVM smart contracts via `pallet-revive`.

### Precompile Address

```
0x0000000000000000000000000000000C010000
```

### Solidity Interface

```solidity
interface IConfidentialAssets {
    function confidentialBalance(uint128 assetId, bytes32 account) external view returns (bytes32);
    function publicKey(bytes32 account) external view returns (bytes32);
    function totalSupply(uint128 assetId) external view returns (bytes32);
}
```

### Usage from Solidity

```solidity
IConfidentialAssets constant CONFIDENTIAL_ASSETS =
    IConfidentialAssets(0x0000000000000000000000000000000C010000);

function getBalance(uint128 assetId, bytes32 account) external view returns (bytes32) {
    return CONFIDENTIAL_ASSETS.confidentialBalance(assetId, account);
}
```

See [book/src/polkavm-precompile.md](./book/src/polkavm-precompile.md) for detailed documentation.

## Testing

### Unit Tests

```bash
# Run all tests
cargo test --workspace

# Run specific pallet tests
cargo test -p pallet-zkhe
cargo test -p pallet-confidential-assets
```

### XCM Simulator Tests

```bash
# Run XCM cross-chain tests
cargo test -p confidential-xcm-tests
```

### Integration Tests with Zombienet

See `zombienet/` directory for network configuration and test scenarios.

```bash
# Install zombienet
npm install -g @zombienet/cli

# Run local network
zombienet spawn zombienet/local.toml

# Run integration tests
cargo test -p integration-tests
```

## Extension Examples

The `book/examples/` directory contains example pallets extending this framework:

| Example | Description |
|---------|-------------|
| `confidential-htlc` | Hash Time-Locked Contracts for atomic swaps |
| `confidential-swaps` | Direct confidential asset swaps |
| `confidential-intents-dex` | Intent-based DEX for confidential trades |
| `confidential-xcm-bridge` | Cross-chain bridge extension |
| `escrow` | Generic escrow patterns |

## References

* [OpenZeppelin Confidential Contracts Standard (IERC7984)](https://github.com/OpenZeppelin/openzeppelin-confidential-contracts/blob/master/contracts/interfaces/IERC7984.sol)
* [Solana Confidential Balances Overview](https://www.solana-program.com/docs/confidential-balances/overview)
* [Polkadot sTPS Benchmarks](https://github.com/paritytech/polkadot-stps)
* [Zombienet Testing Framework](https://github.com/paritytech/zombienet)

## Security Policy

Please report any security issues you find to <security@openzeppelin.com>.
