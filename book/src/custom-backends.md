# Custom Backends

Implement alternative cryptographic backends beyond ZK-ElGamal.

## Overview

The `pallet-confidential-assets` is backend-agnostic. While `pallet-zkhe` provides a ZK-ElGamal implementation, you can implement alternatives:

- **Alternative ZK schemes**: Groth16, Plonk, etc.
- **Fully Homomorphic Encryption (FHE)**: Compute on encrypted data
- **Trusted Execution Environment (TEE)**: Hardware-based privacy
- **Multi-Party Computation (MPC)**: Distributed key management

## ConfidentialBackend Trait

Your backend must implement the `ConfidentialBackend` trait, which defines:

### State Queries
- `total_supply(asset)` - Get total supply commitment
- `balance_of(asset, who)` - Get available balance commitment
- `pending_balance(asset, who)` - Get pending balance commitment
- `public_key(who)` - Get account's public key

### Public Key Management
- `set_public_key(who, pk)` - Register a public key for an account

### Balance Operations
- `transfer_encrypted(asset, from, to, delta_ct, proof)` - Execute confidential transfer
- `claim_encrypted(asset, who, envelope)` - Claim pending transfers
- `mint_encrypted(asset, to, proof)` - Mint new confidential balance (deposit)
- `burn_encrypted(asset, from, amount_ct, proof)` - Burn confidential balance (withdraw)
- `disclose_amount(asset, cipher, who)` - Reveal an encrypted amount

## ZkVerifier Trait

If using ZK proofs, implement the `ZkVerifier` trait:

- `verify_transfer_sent` - Verify sender transfer proof
- `verify_transfer_received` - Verify receiver accept proof
- `verify_mint` - Verify mint/deposit proof
- `verify_burn` - Verify burn/withdraw proof
- `disclose` - Disclose encrypted amount

## Registering Your Backend

Configure your backend in the runtime:

```rust
impl pallet_confidential_assets::Config for Runtime {
    type Backend = MyCustomBackend;
    // ...
}
```

## Considerations by Backend Type

### Alternative ZK Schemes
Different ZK proof systems have trade-offs:
- **Groth16**: Smaller proofs (~200 bytes), faster verification, requires trusted setup
- **Plonk**: Universal setup, larger proofs
- **STARKs**: No trusted setup, largest proofs, quantum-resistant

### FHE Backends
FHE allows computation on encrypted data without proofs, but:
- Operations are significantly slower
- Different encryption formats than ElGamal
- May require interaction with key holders for disclosure

### TEE Backends
Hardware-based privacy through attestation:
- Fast verification (attestation check only)
- Requires trusted hardware assumption
- Different security model than cryptographic proofs

## Migration Between Backends

Migrating between backends is complex and should be avoided if possible. It typically requires:

1. Pausing confidential operations
2. Decrypting all balances (requires key holder cooperation)
3. Re-encrypting under new scheme
4. Updating runtime configuration
5. Resuming operations

## Next Steps

- [ACL & Operators](./acl-operators.md) - Access control options
- [Custom Ramps](./custom-ramps.md) - Custom deposit/withdraw logic
- [Testing Guide](./testing.md) - Test your backend
