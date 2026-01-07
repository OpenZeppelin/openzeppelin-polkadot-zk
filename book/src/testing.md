# Testing Guide

Comprehensive testing strategies for confidential assets.

## Overview

Testing confidential assets requires multiple approaches:

1. **Unit Tests**: Test individual pallet functions
2. **Property Tests**: Verify invariants with random inputs
3. **Integration Tests**: Test pallet interactions
4. **XCM Simulator Tests**: Test cross-chain flows
5. **Vector Tests**: Use pre-generated proofs for deterministic testing

## Test Environment Setup

### Mock Runtime

Set up a mock runtime with:
- `frame_system` for basic runtime functionality
- `pallet_balances` or `pallet_assets` for public asset handling
- `pallet_zkhe` for cryptographic backend
- `pallet_confidential_assets` for the main interface

### Mock Verifier

For unit tests, use a mock verifier that accepts all proofs. This allows testing pallet logic without real cryptographic verification. The mock verifier returns dummy commitments for state updates.

For integration tests with real cryptographic verification, use `zkhe_verifier::ZkheVerifier`.

### Test Accounts

Define test accounts (e.g., ALICE, BOB, CHARLIE) with initial public balances for testing.

## Testing Strategies

### Unit Tests

Test individual extrinsics:
- `set_public_key` - Verify key registration and duplicate prevention
- `deposit` - Verify public to confidential conversion
- `confidential_transfer` - Verify transfer mechanics
- `accept_pending` - Verify pending balance claiming
- `withdraw` - Verify confidential to public conversion

Test error conditions:
- Missing public key
- Insufficient balance
- Invalid proofs (with real verifier)
- Duplicate operations

### Property Tests

Use `proptest` to verify invariants:
- Public key registration succeeds with any valid 32-byte key
- UTXO IDs increment correctly with sequential transfers
- Commitments are always 32 bytes
- Malformed proofs are rejected (with real verifier)

### Integration Tests

Test complete user flows:
1. Register public keys for sender and recipient
2. Deposit public assets to confidential
3. Accept pending deposit
4. Transfer confidentially to recipient
5. Recipient accepts pending transfer
6. Recipient withdraws to public

Verify:
- Public balances decrease/increase correctly
- Events are emitted for each operation
- State transitions are atomic

### Vector Tests

Use pre-generated proofs from `zkhe-vectors` for deterministic testing:
- Verify known-good proofs are accepted
- Verify tampered proofs are rejected
- Verify wrong public keys are rejected
- Verify cross-asset replay is prevented

### XCM Simulator Tests

Test cross-chain confidential transfers:
1. Set up two parachain test environments
2. Register keys on both chains
3. Initiate cross-chain transfer from source
4. Process XCM messages
5. Verify pending balance on destination
6. Accept and verify funds are claimable

## Benchmarking

Generate accurate weights using `frame_benchmarking`:
- Benchmark each extrinsic with varying parameters
- Run on target hardware for production weights
- Include in runtime for accurate fee calculation

## Test Coverage

Use `cargo-tarpaulin` to measure test coverage:
- Target high coverage for pallet logic
- Focus on edge cases and error paths
- Include property tests in coverage analysis

## CI Integration

Automate testing in CI:
- Unit tests on every push
- Property tests with adequate case count
- XCM simulator tests for cross-chain logic
- Vector tests for cryptographic correctness

## Next Steps

- [API Reference](./api.md) - Complete API documentation
- [Client Integration](./client.md) - Build client applications
- [Custom Backends](./custom-backends.md) - Implement alternative backends
