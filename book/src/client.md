# Client Integration

Build client applications that interact with confidential assets.

## Overview

Client applications must handle:

1. **Key Management**: Generate and store ElGamal keypairs
2. **Proof Generation**: Create ZK proofs for operations
3. **Balance Tracking**: Decrypt and track confidential balances
4. **Transaction Building**: Construct and submit extrinsics

## High-Level Flow

### Key Management

1. Generate an ElGamal keypair (secret key + public key)
2. Store the secret key securely (encrypted with user password)
3. Register the public key on-chain via `set_public_key` extrinsic

### Balance Queries

1. Query `AvailableBalanceCommit` for spendable balance commitment
2. Query `PendingBalanceCommit` for incoming transfers awaiting claim
3. Decrypt commitments using the secret key to get plaintext balances

### Deposit (Public to Confidential)

1. Fetch current pending balance commitment and total supply commitment
2. Generate mint proof using `zkhe-prover`
3. Submit `deposit` extrinsic with amount and proof
4. Accept pending balance via `accept_pending` to make funds spendable

### Confidential Transfer

1. Fetch sender's available balance commitment
2. Fetch recipient's pending balance commitment and public key
3. Generate sender proof using `zkhe-prover`
4. Submit `confidential_transfer` extrinsic with encrypted amount and proof
5. Recipient accepts via `accept_pending` to claim funds

### Withdraw (Confidential to Public)

1. Fetch available balance commitment and total supply commitment
2. Generate burn proof using `zkhe-prover`
3. Submit `withdraw` extrinsic with encrypted amount and proof

## Event Subscriptions

Subscribe to pallet events to track:
- `ConfidentialTransfer` - Transfer completed
- `Deposit` - Public to confidential conversion
- `Withdraw` - Confidential to public conversion
- `PendingAccepted` - Pending balance claimed

## Error Handling

Common errors to handle:
- `NoPk` - Account has no registered public key
- `InsufficientBalance` - Insufficient confidential balance
- `ProofVerificationFailed` - ZK proof verification failed
- `PendingNotFound` - No pending balance to claim

## Implementation Notes

- Proof generation is computationally expensive; consider using Web Workers
- Cache decrypted balances with appropriate TTL
- Use batch queries (`api.queryMulti`) for multiple balance lookups
- Handle nonce management for transaction retries

## Next Steps

- [Testing Guide](./testing.md) - Test your client integration
- [API Reference](./api.md) - Complete API documentation
- [Architecture](./architecture.md) - Understand the system design
