# Cryptographic Primitives

Understanding the cryptography behind confidential assets.

## Overview

The ZK-ElGamal scheme combines several cryptographic primitives:

| Primitive | Purpose | Library |
|-----------|---------|---------|
| Pedersen Commitments | Hide balance values | curve25519-dalek |
| Twisted ElGamal | Encrypt amounts for recipients | curve25519-dalek |
| Bulletproofs | Range proofs (0 ≤ v < 2^64) | bulletproofs |
| Merlin Transcripts | Fiat-Shamir challenges | merlin |

## Pedersen Commitments

A commitment to value `v` with randomness `r`:

```
C = v·G + r·H

Where:
  G = Ristretto basepoint
  H = hash_to_point("Zether/PedersenH")
  v = value (secret, 64-bit)
  r = randomness (secret, 256-bit scalar)
  C = commitment (public, 32 bytes compressed)
```

### Properties

**Hiding:** Given only `C`, an adversary cannot determine `v` (assuming `r` is random).

**Binding:** Cannot find `(v', r')` such that `v'·G + r'·H = C` with `v' ≠ v`.

**Homomorphic Addition:**
```
C1 + C2 = (v1·G + r1·H) + (v2·G + r2·H)
        = (v1+v2)·G + (r1+r2)·H
```

This enables balance updates without revealing values.

### Code Example

```rust
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT as G,
    ristretto::RistrettoPoint,
    scalar::Scalar,
};
use sha2::Sha512;

// H generator (domain-separated from G)
fn h_generator() -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(b"Zether/PedersenH")
}

// Create commitment
fn commit(value: u64, randomness: Scalar) -> RistrettoPoint {
    Scalar::from(value) * G + randomness * h_generator()
}

// Verify commitment opening
fn verify_opening(commitment: RistrettoPoint, value: u64, randomness: Scalar) -> bool {
    commitment == commit(value, randomness)
}
```

## Twisted ElGamal Encryption

Encrypts value `v` under public key `pk`:

```
Ciphertext(v) = (C, D)

Where:
  pk = sk·G           (public key)
  sk = secret key scalar
  k  = random encryption scalar
  C  = v·G + k·H      (commitment to v)
  D  = k·pk           (decryption helper)
```

### Decryption

```
v·G = C - (sk⁻¹)·D·sk
    = C - k·pk·(sk⁻¹)·sk
    = C - k·G
    = v·G + k·H - k·G
    = v·G

Then brute-force v from v·G (feasible for 64-bit values).
```

### Code Example

```rust
struct ElGamalCiphertext {
    c: RistrettoPoint,  // Commitment
    d: RistrettoPoint,  // Decryption helper
}

fn encrypt(value: u64, pk: RistrettoPoint, k: Scalar) -> ElGamalCiphertext {
    ElGamalCiphertext {
        c: Scalar::from(value) * G + k * h_generator(),
        d: k * pk,
    }
}

fn decrypt_commitment(ct: &ElGamalCiphertext, sk: Scalar) -> RistrettoPoint {
    // Returns v·G; caller must solve ECDLP for small v
    ct.c - sk.invert() * ct.d * sk
}
```

## Bulletproofs Range Proofs

Proves that a committed value is in range `[0, 2^n)` without revealing it.

### Structure

```
RangeProof for commitment C = v·G + r·H:
  - Proves: 0 ≤ v < 2^64
  - Size: ~700 bytes (logarithmic in range)
  - Verification: O(n) curve operations
```

### Aggregation

Multiple range proofs can be aggregated:

```
Single proof:  ~700 bytes
2 proofs:      ~800 bytes (not 1400)
4 proofs:      ~900 bytes
```

### Code Example

```rust
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use merlin::Transcript;

fn create_range_proof(value: u64, blinding: Scalar) -> Vec<u8> {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(64, 1);

    let mut transcript = Transcript::new(b"RangeProof");

    let (proof, _) = RangeProof::prove_single(
        &bp_gens,
        &pc_gens,
        &mut transcript,
        value,
        &blinding,
        64,  // 64-bit range
    ).expect("proof creation");

    proof.to_bytes()
}

fn verify_range_proof(commitment: CompressedRistretto, proof_bytes: &[u8]) -> bool {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(64, 1);

    let proof = RangeProof::from_bytes(proof_bytes).expect("parse");
    let mut transcript = Transcript::new(b"RangeProof");

    proof.verify_single(&bp_gens, &pc_gens, &mut transcript, &commitment, 64).is_ok()
}
```

## Merlin Transcripts

Fiat-Shamir transformation for non-interactive proofs.

### Usage

```rust
use merlin::Transcript;

fn create_transcript(context: &[u8]) -> Transcript {
    let mut t = Transcript::new(b"ZkHE");
    t.append_message(b"context", context);
    t
}

// Add proof elements
transcript.append_message(b"commitment", &commitment.compress().to_bytes());

// Generate challenge
let mut challenge_bytes = [0u8; 64];
transcript.challenge_bytes(b"challenge", &mut challenge_bytes);
let challenge = Scalar::from_bytes_mod_order_wide(&challenge_bytes);
```

## Link Proofs

Proves that a ciphertext encrypts the same value as a commitment.

### Statement

```
Given:
  C = v·G + r·H          (Pedersen commitment)
  (E, D) = (v·G + k·H, k·pk)  (ElGamal ciphertext)

Prove:
  C and E commit to the same v
  Without revealing v, r, or k
```

### Proof Structure

```
Link proof contains:
  - Challenge scalar c
  - Response scalars for v, r, k
  - Verification equations bind C and E
```

## Proof Bundle Format

The sender bundle contains:

```
SenderBundle (serialized):
┌─────────────────────────────────────────┐
│ delta_commitment [32 bytes]             │  ΔC
├─────────────────────────────────────────┤
│ from_new_commitment [32 bytes]          │  Sender's new balance
├─────────────────────────────────────────┤
│ to_new_pending [32 bytes]               │  Receiver's new pending
├─────────────────────────────────────────┤
│ range_proof [~700 bytes]                │  Δv ∈ [0, 2^64)
├─────────────────────────────────────────┤
│ link_proof [~200 bytes]                 │  Δct encrypts Δv
├─────────────────────────────────────────┤
│ balance_proof [~200 bytes]              │  Sender has funds
└─────────────────────────────────────────┘
Total: ~1200 bytes typical
```

## Security Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| Curve | Ristretto255 | ~128-bit security |
| Scalar size | 256 bits | Full curve order |
| Range proof bits | 64 | Supports u64 values |
| Hash function | SHA-512 | For generators and challenges |

## Transcript Context

All proofs include a context binding:

```rust
fn transcript_context(asset_id: &[u8], network_id: [u8; 32]) -> Vec<u8> {
    let mut ctx = Vec::with_capacity(32 + asset_id.len());
    ctx.extend_from_slice(&network_id);
    ctx.extend_from_slice(asset_id);
    ctx
}
```

This prevents proof replay across assets or networks.

## Performance

Typical operation times (M1 Mac):

| Operation | Time |
|-----------|------|
| Commitment creation | 50 μs |
| Range proof creation | 15 ms |
| Range proof verification | 3 ms |
| Link proof creation | 2 ms |
| Link proof verification | 1 ms |
| Full sender proof | 20 ms |
| Full sender verification | 5 ms |

## References

- [Bulletproofs paper](https://eprint.iacr.org/2017/1066)
- [Ristretto group](https://ristretto.group/)
- [Merlin transcripts](https://merlin.cool/)
- [Solana ZK SDK](https://docs.rs/solana-zk-sdk/)
