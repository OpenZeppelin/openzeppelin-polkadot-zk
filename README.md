# OpenZeppelin Polkadot Zero Knowledge

**WARNING: This is beta software that has NOT been audited and is not ready for production. Use at your own risk!**

Rust libraries for executing and verifying computations on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

## Pallets

1. `pallet-confidential-assets`: confidential transfers with encrypted balances generic over crypto backend (ZK, FHE, TEE)
2. `pallet-confidential-bridge`: confidential cross-chain transfers
3. `pallet-confidential-escrow`: confidential asset escrow management
4. `pallet-zkhe`: encrypted balances stored on-chain post verification by generic ZK backend assigned at runtime
5. `pallet-operators`: operator permissions registry for IERC 7984
6. `pallet-acl`: ACL permissions storage for IERC 7984

Note:
- `pallet-confidential-assets` public interface follows the Confidential Transfers Standard (ERC 7984) by OpenZeppelin
- `pallet-confidential-assets` has a generic backend for its cryptography
	- generic backend is assigned at runtime to `Zkhe`, runtime instance of `pallet_zkhe` representing the runtime's implementation of `pallet_zkhe::Config`
- `pallet-zkhe` has a generic verifier for its cryptography
	- generic verifier assigned at runtime to `zkhe_verifier::ZkheVerifier`

## Examples

See [examples](./book/examples/) for pallets that build on top of `pallet-confidential-assets`.

## References

* [OpenZeppelin Confidential Contracts Standard](https://github.com/OpenZeppelin/openzeppelin-confidential-contracts/blob/master/contracts/interfaces/IERC7984.sol)
* [Solana Confidential Balances Overview](https://www.solana-program.com/docs/confidential-balances/overview)
