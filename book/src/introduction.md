# Polkadot Confidential Payments

A Polkadot implementation supporting multiple confidential assets in a single pallet (like `pallet-assets`), generic over a cryptographic backend. The included ZK-ElGamal backend is a variation on the [Solana Confidential Token](https://solana.com/docs/tokens/extensions/confidential-transfer) standard that uses per-recipient UTXO sets to efficiently queue received funds before they are claimed via `confidential_claim` or spent directly via `accept_pending_and_transfer`.

## What is This?

A complete solution for adding confidential (private amount) transfers to your Substrate-based blockchain:

- **Amounts are hidden**: Transfer values are encrypted and only visible to sender and receiver
- **Addresses are public**: Sender and receiver addresses remain visible for compliance
- **Fully verified on-chain**: Zero-knowledge proofs ensure correctness without revealing amounts
- **Cross-chain ready**: Built-in support for confidential XCM transfers between parachains

## Key Components

| Component | Description |
|-----------|-------------|
| `pallet-confidential-assets` | User-facing API: `deposit`, `withdraw`, `confidential_transfer`, `confidential_claim` |
| `pallet-zkhe` | ZK backend with UTXO storage: `accept_pending`, `accept_pending_and_transfer` |
| `pallet-confidential-bridge` | Cross-chain confidential transfers via XCM |
| `pallet-confidential-escrow` | Escrow management for cross-chain operations |
| `zkhe/prover` | Client-side proof generation (std) |
| `zkhe/verifier` | On-chain proof verification (no_std) |

## Cryptography

This implementation uses the same cryptographic primitives as [Solana Confidential Transfers](https://solana.com/docs/tokens/extensions/confidential-transfer):

- **Pedersen Commitments** for balance hiding
- **Twisted ElGamal** for amount encryption
- **Bulletproofs** for range proofs

See [Cryptographic Primitives](./crypto.md) for details and links to Solana documentation.

## Design Principles

1. **Privacy, not anonymity**: Addresses are public, amounts are private
2. **Separation of concerns**: Confidential and public assets have separate paths
3. **On-chain verification**: All state changes are ZK-verified
4. **Extensibility**: Backend-agnostic design supports alternative cryptographic schemes

## Getting Started

- [Quick Start](./quickstart.md) - Get running in 5 minutes
- [Architecture Overview](./architecture.md) - Understand the system design
- [Asset Hub Integration](./asset-hub.md) - Deploy to Asset Hub
