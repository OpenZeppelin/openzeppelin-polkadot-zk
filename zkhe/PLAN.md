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

#### Property Testing (DONE)

Implemented property tests verify:

**pallet-zkhe** (6 property tests):
- [x] `prop_transfer_succeeds_with_valid_pks` - transfer works with valid PKs
- [x] `prop_transfer_fails_without_sender_pk` - transfer fails without sender PK
- [x] `prop_sequential_transfers_increment_utxo_ids` - UTXO IDs increment correctly
- [x] `prop_accept_pending_rejects_malformed_envelope` - envelope validation
- [x] `prop_mint_creates_valid_pending_state` - mint creates correct state
- [x] `prop_burn_updates_commits_correctly` - burn updates commitments

**pallet-confidential-assets** (8 property tests):
- [x] `prop_set_public_key_always_succeeds` - PK setting always works
- [x] `prop_deposit_succeeds_with_pk` - deposit works with valid PK
- [x] `prop_confidential_transfer_succeeds` - transfer between distinct accounts
- [x] `prop_withdraw_succeeds` - withdraw with disclosed amount
- [x] `prop_disclose_amount_succeeds` - disclosure emits correct event
- [x] `prop_transfer_from_unauthorized_fails` - unauthorized transfer rejected
- [x] `prop_transfer_from_by_owner_succeeds` - owner can transfer

#### XCM Integration (DONE)

- [x] Created `xcm/src/vector_tests.rs` using `zkhe_vectors::*` constants
- [x] Added 7 deterministic test scenarios:
  - `verify_transfer_sent_with_vectors`
  - `verify_transfer_received_with_vectors`
  - `verify_mint_with_vectors`
  - `verify_burn_with_vectors`
  - `full_transfer_roundtrip_with_vectors`
  - `tampered_bundle_rejected`
  - `wrong_pk_rejected`

#### Documentation (DONE)

Created comprehensive mdbook documentation at `book/`:

- [x] `introduction.md` - Project overview and features
- [x] `quickstart.md` - 5-minute getting started guide
- [x] `architecture.md` - System design with diagrams
- [x] `asset-hub.md` - Asset Hub integration guide
- [x] `configuration.md` - Complete pallet configuration reference
- [x] `crypto.md` - Cryptographic primitives documentation
- [x] `runtime-integration.md` - Runtime integration guide
- [x] `xcm-setup.md` - XCM cross-chain configuration
- [x] `custom-backends.md` - Alternative backend implementation
- [x] `acl-operators.md` - Access control and operators
- [x] `custom-ramps.md` - Custom ramp implementation
- [x] `client.md` - Client integration guide (JS/TS SDK)
- [x] `testing.md` - Testing strategies and examples
- [x] `api.md` - Complete API reference

#### Future Work

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
