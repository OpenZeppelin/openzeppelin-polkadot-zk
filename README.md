# OpenZeppelin Polkadot Zero Knowledge

Rust libraries for executing and verifying computations on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

Repo Overview:
```
\pallets
	\confidential-assets:
		confidential transfers with encrypted balances generic over crypto backend (ZK, FHE, TEE)
	\confidential-xcm-bridge:
		cross-chain atomic swaps for confidential assets
	\confidential-htlc:
		hashed timelocks over confidential asset escrow management
	\confidential-escrow:
		confidential asset escrow management
	\confidential-swaps:
		native swaps between confidential assets
	\confidential-intents-dex:
		confidential assets DEX using intents for order matching
	\zkhe:
		encrypted balances verified and stored on-chain generic over verifier backend (ZK El-Gamal)
    \htlc:
		hashed timelocks over public asset escrow management
	\escrow:
		public asset escrow management
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
```

- `pallet-confidential-assets` public interface follows the Confidential Transfers Standard (ERC 7984) by OpenZeppelin
- `pallet-confidential-assets` has a generic backend for its cryptography
	- generic backend is assigned at runtime to `Zkhe`, runtime instance of `pallet_zkhe` representing the runtime's implementation of `pallet_zkhe::Config`
- `pallet-zkhe` has a generic verifier for its cryptography
	- generic verifier assigned at runtime to `zkhe_verifier::ZkheVerifier`

## References

* [OpenZeppelin Confidential Contracts Standard](https://github.com/OpenZeppelin/openzeppelin-confidential-contracts/blob/master/contracts/interfaces/IERC7984.sol)
* [Solana Confidential Balances Overview](https://www.solana-program.com/docs/confidential-balances/overview)

## Future Work

The following modules represent potential future extensions of the confidential framework. They are listed here for repository scope planning and naming consistency only.
•	pallet-confidential-airdrop
•	pallet-confidential-uniswap-dex
•	pallet-confidential-perp-dex
•	pallet-confidential-auctions
•	pallet-confidential-voting

Primitive cryptographic pallets supporting these extensions may include:
•	pallet-merkle-tree
•	pallet-kite-vote
•	pallet-kite-snapshot