# OpenZeppelin Polkadot Zero Knowledge

Rust libraries for executing and verifying computations on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

## Confidential Assets

```
\pallets
	\confidential-assets:
		confidential transfers with encrypted balances generic over crypto backend (ZK, FHE, TEE)
	\confidential-xcm-bridge:
		cross-chain atomic swaps for confidential assets
	\zkhe:
		encrypted balances verified and stored on-chain generic over verifier backend (ZK El-Gamal)
	\confidential-swaps:
		native swaps of confidential assets
    \hash-timelock:
	    cross-chain atomic swaps
	\operators:
	    operator permissions registry for IERC 7984
	\acl:
		ACL permissions storage for IERC 7984
\primitives
	\zkhe:
		shared primitives between prover and verifier
	\confidential-assets:
		shared primitives between pallets
\zkhe-prover
	off-chain library to construct ZK proofs expected by confidential pallet extrinsics
\zkhe-verifier
	no_std library to verify ZK proofs on-chain
\docs
    WIP docs/notes
```

- `pallet-confidential-assets` public interface follows the Confidential Transfers Standard (ERC 7984) by OpenZeppelin
- `pallet-confidential-assets` has a generic backend for its cryptography
	- generic backend is assigned at runtime to `Zkhe`, runtime instance of `pallet_zkhe` representing the runtime's implementation of `pallet_zkhe::Config`
- `pallet-zkhe` has a generic verifier for its cryptography
	- generic verifier assigned at runtime to `zkhe_verifier::ZkheVerifier`