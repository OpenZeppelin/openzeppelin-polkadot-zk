# Polkadot Confidential Payments

[![Lint and Test](https://github.com/OpenZeppelin/openzeppelin-polkadot-zk/actions/workflows/ci.yml/badge.svg)](https://github.com/OpenZeppelin/openzeppelin-polkadot-zk/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

> **Experimental. Not audited.**

See the [Sub0 Buenos Aires 2025 talk](https://youtu.be/WuVVmCruJOo) for an overview.

Confidential multi-asset transfers for Polkadot. Uses a variation on [Solana Confidential Tokens](https://solana.com/docs/tokens/extensions/confidential-transfer) with per-recipient UTXO sets for pending funds. Claim via `confidential_claim` or spend directly via `accept_pending_and_transfer`.

## Components

| Pallet | Purpose |
|--------|---------|
| `pallet-confidential-assets` | User API: `deposit`, `withdraw`, `confidential_transfer`, `confidential_claim` |
| `pallet-zkhe` | ZK backend: `accept_pending`, `accept_pending_and_transfer` |
| `pallet-confidential-bridge` | Cross-chain via HRMP |
| `pallet-confidential-escrow` | Atomic swaps |

| Crate | Purpose |
|-------|---------|
| `zkhe-prover` | Off-chain proof generation |
| `zkhe-verifier` | On-chain verification (no_std) |

## Cryptography

Same primitives as [Solana Confidential Transfers](https://www.solana-program.com/docs/confidential-balances/zkps): Pedersen commitments, twisted ElGamal, Bulletproofs.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo build --release
cargo test --workspace
```

## Docs

- [Architecture](./book/src/architecture.md)
- [Crypto](./book/src/crypto.md)
- [Configuration](./book/src/configuration.md)

## References

- [Solana Confidential Transfer](https://solana.com/docs/tokens/extensions/confidential-transfer)
- [Solana ZK Proofs](https://www.solana-program.com/docs/confidential-balances/zkps)

## License

[GPL v3](LICENSE)
