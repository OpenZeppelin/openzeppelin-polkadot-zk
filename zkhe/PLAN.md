# ZKHE (Zero-Knowledge Homomorphic Encryption) Development Plan

This document tracks improvements, issues, and development tasks for the ZKHE subsystem.

## Directory Structure

```
zkhe/
├── prover/     # std-only client-side proof generation
├── verifier/   # no_std on-chain verification
├── vectors/    # Generated test vectors for deterministic testing
└── PLAN.md     # This file
```

## Code Review Observations

### Prover (`zkhe/prover/`)

**Strengths:**
- Clean separation of sender/receiver/mint/burn proof generation
- Well-documented protocol flow (two-phase transfer)
- Deterministic RNG for reproducible tests
- SDK interop validation with Solana ZK SDK

**Potential Improvements:**

1. **Error Handling** (`lib.rs:45-51`)
   - [ ] `ProverError` could include more context (e.g., which commitment failed)
   - [ ] Consider adding `InvalidInput` variant for clearer error messages

2. **Code Duplication** (`lib.rs:53-56`, `lib.rs:142-150`)
   - [ ] `pedersen_h_generator()` is duplicated in prover and verifier
   - [ ] Consider extracting to `zkhe-primitives` for shared use

3. **Scalar Generation** (`lib.rs:213-217`)
   - [ ] Random scalars are generated from `u64` which limits entropy
   - [ ] Consider using full 256-bit scalar generation for production security

4. **SDK Version Coupling** (`lib.rs:42-43`)
   - [ ] `solana_zk_sdk` interop check could be feature-gated
   - [ ] Not all deployments need Solana compatibility

5. **Documentation**
   - [ ] Add rustdoc examples for each public function
   - [ ] Document the proof format byte layouts more clearly

### Verifier (`zkhe/verifier/`)

**Strengths:**
- `no_std` compatible for on-chain use
- Comprehensive verification of link proofs and range proofs
- Clean trait implementation (`ZkVerifier`)

**Potential Improvements:**

1. **Error Type** (`lib.rs:29-30`)
   - [ ] Replace `()` error type with proper enum
   - [ ] Would enable better debugging and error handling

2. **Debug Output** (`range.rs:8-32`)
   - [ ] Debug macros compile even without `std`; consider cfg(debug_assertions)
   - [ ] `hex()` function allocates unnecessarily in non-debug builds

3. **Transcript Context** (`lib.rs:592-596`)
   - [ ] `transcript_context_bytes` returns `Vec<u8>` but 32 bytes would suffice
   - [ ] Minor allocation that could be avoided

4. **Hardcoded Network ID** (`lib.rs:51`, `lib.rs:150`, etc.)
   - [ ] Network ID is hardcoded to `[0u8; 32]` throughout
   - [ ] Should be parameterized for multi-network support

5. **Benchmark Infrastructure** (`per-block/`)
   - [ ] Paths to criterion results are relative and fragile
   - [ ] Consider using env vars or config file

### Vectors (`zkhe/vectors/`)

**Observations:**
- Generated deterministically from prover
- Covers transfer, accept, mint, and burn scenarios

**Potential Improvements:**

1. **Vector Coverage**
   - [ ] Add edge case vectors (zero values, max values)
   - [ ] Add negative test vectors (malformed proofs)
   - [ ] Add vectors for multi-asset scenarios

2. **Generation Process**
   - [ ] Document regeneration procedure
   - [ ] Add CI check to verify vectors match prover output

---

## Task Tracking

### Completed

- [x] Move zkhe-* crates to zkhe/ directory
- [x] Rename directories (prover, verifier, vectors)
- [x] Update all path references in Cargo.toml files
- [x] Verify compilation and tests pass
- [x] Add property test coverage for pallet-zkhe (6 property tests)
- [x] Add property test coverage for pallet-confidential-assets (8 property tests)
- [x] Create vector_tests.rs in XCM using zkhe-vectors (7 deterministic tests)

### In Progress

None currently.

### Planned

#### Property Testing

Property tests should verify:

**Runtime:**
- [ ] Pallet integration doesn't panic with arbitrary valid inputs
- [ ] State transitions are consistent (commitments balance)
- [ ] Weight calculations are accurate

**pallet-confidential-assets:**
- [ ] deposit/withdraw roundtrip preserves invariants
- [ ] confidential_transfer maintains commitment sums
- [ ] claim properly consumes pending UTXOs

**pallet-zkhe:**
- [ ] transfer + accept_pending roundtrip
- [ ] mint_encrypted creates valid pending state
- [ ] burn_encrypted properly decrements commitments

#### XCM Integration

- [ ] Use `zkhe_vectors::*` constants in XCM tests
- [ ] Remove hardcoded proof generation in `confidential_xcm_transfer.rs`
- [ ] Add deterministic test scenarios matching vectors

---

## Architecture Notes

### Proof Flow

```
┌─────────────────┐      ┌─────────────────┐
│  Sender (std)   │      │ Receiver (std)  │
│                 │      │                 │
│ prove_sender_   │──────▶│ prove_receiver_ │
│ transfer()      │ Δct  │ accept()        │
└────────┬────────┘      └────────┬────────┘
         │                        │
         │ bundle                 │ envelope
         ▼                        ▼
┌─────────────────────────────────────────┐
│         On-chain (no_std)               │
│                                         │
│  verify_transfer_sent() ──────────────▶ │
│  verify_transfer_received() ◀────────── │
└─────────────────────────────────────────┘
```

### Commitment Types

- **Available**: Spendable balance (v*G + r*H)
- **Pending**: Incoming transfers awaiting claim
- **Total Supply**: Sum of all confidential balances
- **Delta (ΔC)**: Transfer amount commitment

### Key Cryptographic Components

1. **ElGamal Encryption**: Amount confidentiality
2. **Pedersen Commitments**: Balance hiding
3. **Bulletproofs**: 64-bit range proofs
4. **Merlin Transcripts**: Fiat-Shamir challenges

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bulletproofs | 4.x | Range proofs |
| curve25519-dalek | 4.1.3 | Elliptic curve ops |
| curve25519-dalek-ng | 4.1.1 | Bulletproofs compat |
| merlin | 3.x | Transcript construction |
| solana-zk-sdk | 4.x | SDK interop (prover only) |

---

## Notes

- The prover uses Rust 2024 edition features
- Verifier must remain `no_std` compatible for WASM
- Test vectors should be regenerated when proof format changes
