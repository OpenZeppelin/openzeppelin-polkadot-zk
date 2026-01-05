# Cryptographic Primitives

This framework uses the same cryptographic approach as [Solana Confidential Transfers](https://solana.com/docs/tokens/extensions/confidential-transfer). For detailed cryptographic specifications, refer to the Solana documentation.

## Overview

| Primitive | Purpose | Solana Documentation |
|-----------|---------|---------------------|
| Pedersen Commitments | Hide balance values | [ZK Proofs](https://www.solana-program.com/docs/confidential-balances/zkps) |
| Twisted ElGamal | Encrypt amounts for recipients | [ZK ElGamal Proof Program](https://docs.anza.xyz/runtime/zk-elgamal-proof) |
| Bulletproofs | Range proofs (0 ≤ v < 2^64) | [ZK Proofs](https://www.solana-program.com/docs/confidential-balances/zkps) |
| Fiat-Shamir | Non-interactive proof generation | [ZK Proofs](https://www.solana-program.com/docs/confidential-balances/zkps) |

## Pedersen Commitments

Pedersen commitments allow hiding a value while enabling verification of arithmetic operations on committed values. They are computationally binding (cannot change the committed value) and perfectly hiding (reveal nothing about the value). The homomorphic property allows adding commitments without knowing the underlying values.

See: [Solana ZK Proofs - Pedersen Commitments](https://www.solana-program.com/docs/confidential-balances/zkps)

## Twisted ElGamal Encryption

Twisted ElGamal is a public-key encryption scheme that encrypts values under a recipient's public key. Only the holder of the corresponding secret key can decrypt. The "twisted" variant is optimized for use with Pedersen commitments, allowing encrypted values to be verified without decryption.

See: [Solana ZK ElGamal Proof Program](https://docs.anza.xyz/runtime/zk-elgamal-proof)

## Bulletproofs Range Proofs

Range proofs demonstrate that a committed value lies within a valid range (0 to 2^64) without revealing the actual value. This prevents negative balances and overflow attacks. Bulletproofs provide efficient, compact range proofs with logarithmic proof size.

See: [Solana ZK Proofs - Range Proofs](https://www.solana-program.com/docs/confidential-balances/zkps)

## Link Proofs

Link proofs (also called equality proofs or ciphertext-commitment equality proofs) demonstrate that an ElGamal ciphertext encrypts the same value as a Pedersen commitment. This ensures consistency between encrypted transfers and balance updates.

See: [Solana ZK ElGamal Proof Program - Proof Types](https://docs.anza.xyz/runtime/zk-elgamal-proof)

## Transcript Context

All proofs include domain separation via transcript context binding. This prevents proof replay across different assets or networks by including the asset ID and network ID in the proof generation.

## Further Reading

- [Solana Confidential Transfer Overview](https://solana.com/docs/tokens/extensions/confidential-transfer)
- [Solana Confidential Balances Protocol](https://www.solana-program.com/docs/confidential-balances/overview)
- [Bulletproofs Paper](https://eprint.iacr.org/2017/1066)
- [Ristretto Group](https://ristretto.group/)
- [Solana ZK SDK (Rust)](https://docs.rs/solana-zk-sdk/)
