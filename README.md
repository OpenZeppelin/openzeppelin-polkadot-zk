# Polkadot Confidential Assets Framework

**WARNING: This is beta software that has NOT been audited and is NOT ready for production. Use at your own risk!**

Rust libraries for verifying computation executed on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

**Pallets**
1. `pallet-confidential-assets`: confidential transfers interface following IERC7984 by OpenZeppelin, generic over cryptographic backend (ZK, FHE, TEE)
2. `pallet-confidential-bridge`: confidential cross-chain transfers
3. `pallet-confidential-escrow`: confidential asset escrow management
4. `pallet-zkhe`: encrypted balances stored on-chain post verification by generic ZK backend
5. `pallet-operators`: operator permissions registry for IERC 7984
6. `pallet-acl`: ACL permissions storage for IERC 7984

**Prover+Verifier**
1. `zkhe-prover`: client library to run off-chain for generating valid Zero Knowledge Proofs expected by on-chain verifier
2. `zkhe-verifier`: no_std verifier library to run on-chain for verifying valid Zero Knowledge Proofs expected by off-chain prover

## Examples

Read [the extension pallets](./book/examples/) for examples leveraging and extending the confidential assets framework.

## References

* [OpenZeppelin Confidential Contracts Standard](https://github.com/OpenZeppelin/openzeppelin-confidential-contracts/blob/master/contracts/interfaces/IERC7984.sol)
* [Solana Confidential Balances Overview](https://www.solana-program.com/docs/confidential-balances/overview)
