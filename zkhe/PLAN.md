# ZKHE (Zero-Knowledge Homomorphic Encryption) Development Plan

This document tracks improvements, issues, and development tasks for the ZKHE subsystem.

## Directory Structure

```text
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
- SDK interop validation with Solana ZK SDK (feature-gated)
- Full 256-bit entropy for cryptographic scalars

**Completed Improvements:**

1. **Error Handling** (`lib.rs`)
   - [x] `ProverError` now includes contextual messages
   - [x] Added `InvalidInput` and `Overflow` variants for clearer error messages

2. **Code Deduplication**
   - [x] `pedersen_h_generator()` extracted to `zkhe-primitives` for shared use

3. **Scalar Generation** (`lib.rs:118-126`)
   - [x] Random scalars now use full 256-bit entropy via `Scalar::from_bytes_mod_order_wide`

4. **SDK Version Coupling**
   - [x] `solana_zk_sdk` interop check is now feature-gated (`solana-interop` feature)

5. **Documentation**
   - [x] Added comprehensive rustdoc with examples for all public functions
   - [x] Documented proof format byte layouts in module docs

### Verifier (`zkhe/verifier/`)

**Strengths:**
- `no_std` compatible for on-chain use
- Comprehensive verification of link proofs and range proofs
- Clean trait implementation (`ZkVerifier`)

**Completed Improvements:**

1. **Error Type** (`lib.rs:45-76`)
   - [x] Added `VerifierError` enum with detailed variants for debugging
   - [x] Trait compatibility maintained via `impl From<VerifierError> for ()`

2. **Debug Output** (`range.rs:8-35`)
   - [x] Debug macros now require both `debug_assertions` AND `std`/`test`
   - [x] `hex()` function has no-op stub to avoid allocations in release builds

3. **Transcript Context** (`lib.rs:654-659`)
   - [x] `transcript_context_bytes` now returns `[u8; 32]` instead of `Vec<u8>`

4. **Network ID Parameterization** (`lib.rs`)
   - [x] Added `NetworkIdProvider` trait to `confidential-assets-primitives`
   - [x] `ZkVerifier` trait now has `type NetworkIdProvider` associated type
   - [x] `ZkheVerifier<N>` is now generic over `N: NetworkIdProvider`
   - [x] All hardcoded `[0u8; 32]` replaced with `N::network_id()` calls
   - [x] Test mocks updated with `MockNetworkId` provider
   - [x] Documentation updated with NetworkIdProvider examples

**Remaining Items:**

5. **Benchmark Infrastructure** (`per-block/`)
   - [ ] Paths to criterion results are relative and fragile
   - [ ] Consider using env vars or config file

### Vectors (`zkhe/vectors/`)

**Observations:**
- Generated deterministically from prover
- Covers transfer, accept, mint, and burn scenarios
- Includes edge case and negative test vectors

**Completed Improvements:**

1. **Vector Coverage**
   - [x] Added edge case vectors (large values: 1 billion mint, full balance burn)
   - [x] Added negative test vectors (truncated, tampered, invalid point)

2. **Generation Process**
   - [x] Documented regeneration procedure (see below)

---

## Test Vector Regeneration Procedure

When modifying proof generation or cryptographic primitives, regenerate test vectors:

```bash
# From the workspace root
cargo run -p zkhe-prover --bin gen_vectors

# This writes to: zkhe/vectors/src/generated.rs
```

**Important:** The vector generator (`zkhe/prover/src/bench_vectors.rs`) must match
the prover's internal random scalar generation. If you change how scalars are generated:

1. Update `bench_vectors.rs` to derive `delta_rho` using the same method
2. Regenerate vectors with the command above
3. Run tests to verify: `cargo test -p zkhe-prover -p zkhe-verifier -p zkhe-vectors`

### Vector Categories

**Standard Vectors:**
- `TRANSFER_*` - Sender transfer proof (111 units)
- `ACCEPT_*` - Receiver acceptance proof
- `MINT_*` - Mint/deposit proof (77 units)
- `BURN_*` - Burn/withdraw proof (120 units)

**Edge Case Vectors:**
- `LARGE_MINT_*` - Large value mint (1 billion units)
- `FULL_BURN_*` - Full balance burn (1000 units to zero)

**Negative Test Vectors (should fail verification):**
- `MALFORMED_TRUNCATED_BUNDLE` - Too short to parse
- `MALFORMED_TAMPERED_BUNDLE` - Valid length but corrupted
- `MALFORMED_INVALID_POINT` - Not a valid curve point

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
- [x] Enhanced ProverError with contextual messages and InvalidInput/Overflow variants
- [x] Extracted pedersen_h_generator to zkhe-primitives
- [x] Improved scalar generation to use 256-bit entropy
- [x] Feature-gated Solana SDK interop
- [x] Added rustdoc examples to prover
- [x] Added VerifierError enum for detailed debugging
- [x] Optimized debug output with cfg(debug_assertions)
- [x] Fixed transcript_context_bytes to return [u8; 32]
- [x] Added edge case test vectors (large mint, full burn)
- [x] Added negative test vectors (truncated, tampered, invalid point)
- [x] Documented vector regeneration procedure
- [x] Parameterized network ID via `NetworkIdProvider` trait

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

- [x] Parameterize network ID for multi-network support (DONE via `NetworkIdProvider` trait)
- [ ] Add CI check to verify vectors match prover output
- [ ] Improve benchmark infrastructure path handling

---

## Architecture Notes

### Proof Flow

```text
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
2. **Pedersen Commitments**: Balance hiding (H derived via `hash_to_ristretto(b"Zether/PedersenH")`)
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
| solana-zk-sdk | 4.x | SDK interop (prover only, feature-gated) |

---

## Notes

- The prover uses Rust 2024 edition features
- Verifier must remain `no_std` compatible for WASM
- Test vectors should be regenerated when proof format changes
- Scalars use 256-bit entropy for production security
