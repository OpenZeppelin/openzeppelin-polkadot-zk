# Confidential Assets for Polkadot

This framework enables **confidential asset transfers** on Polkadot parachains, with a primary focus on **Asset Hub** integration. It implements the [ERC-7984 Confidential Token Standard](https://eips.ethereum.org/EIPS/eip-7984) using Zero-Knowledge proofs based on the ZK-ElGamal scheme.

## What is This?

A complete solution for adding confidential (private amount) transfers to your Substrate-based blockchain:

- **Amounts are hidden**: Transfer values are encrypted and only visible to sender and receiver
- **Addresses are public**: Sender and receiver addresses remain visible for compliance
- **Fully verified on-chain**: Zero-knowledge proofs ensure correctness without revealing amounts
- **Cross-chain ready**: Built-in support for confidential XCM transfers between parachains

## Target Use Case: Asset Hub

This framework is designed to integrate with **Polkadot Asset Hub**, enabling:

- Confidential transfers of DOT, USDT, USDC, and other Asset Hub assets
- Privacy-preserving DeFi applications
- Cross-chain confidential transfers via XCM
- Compliance-friendly design (public addresses, private amounts)

## Key Components

| Component | Description |
|-----------|-------------|
| `pallet-confidential-assets` | Main interface following ERC-7984 |
| `pallet-zkhe` | ZK-ElGamal backend for encrypted balance storage |
| `pallet-confidential-bridge` | Cross-chain confidential transfers via XCM |
| `pallet-confidential-escrow` | Escrow management for cross-chain operations |
| `zkhe/prover` | Client-side proof generation (std) |
| `zkhe/verifier` | On-chain proof verification (no_std) |
| `zkhe/vectors` | Pre-generated test vectors |

## Quick Example

```rust
// Client: Generate transfer proof
let proof = zkhe_prover::prove_sender_transfer(&SenderInput {
    asset_id: b"DOT".to_vec(),
    sender_pk,
    receiver_pk,
    from_old_c: sender_balance_commitment,
    delta_value: 100, // Transfer 100 units (hidden)
    // ...
})?;

// On-chain: Execute confidential transfer
ConfidentialAssets::confidential_transfer(
    origin,
    asset_id,
    recipient,
    encrypted_amount,
    proof,
)?;
```

## Design Principles

1. **Privacy, not anonymity**: Addresses are public, amounts are private
2. **Separation of concerns**: Confidential and public assets have separate paths
3. **On-chain verification**: All state changes are ZK-verified
4. **Extensibility**: Backend-agnostic design supports ZK, FHE, or TEE

## Getting Started

- [Quick Start](./quickstart.md) - Get running in 5 minutes
- [Asset Hub Integration](./asset-hub.md) - Deploy to Asset Hub
- [Architecture Overview](./architecture.md) - Understand the system design
