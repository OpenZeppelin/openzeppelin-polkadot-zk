# Custom Backends

Implement alternative cryptographic backends beyond ZK-ElGamal.

## Overview

The `pallet-confidential-assets` is backend-agnostic. While `pallet-zkhe` provides a ZK-ElGamal implementation, you can implement alternatives:

- **Fully Homomorphic Encryption (FHE)**: Compute on encrypted data
- **Trusted Execution Environment (TEE)**: Hardware-based privacy
- **Multi-Party Computation (MPC)**: Distributed key management
- **Alternative ZK schemes**: Groth16, Plonk, etc.

## ConfidentialBackend Trait

Your backend must implement this trait:

```rust
pub trait ConfidentialBackend<AccountId, AssetId, Balance> {
    type Error;

    // === State Queries ===

    /// Get total supply commitment for an asset
    fn total_supply(asset: AssetId) -> Commitment;

    /// Get account's available balance commitment
    fn balance_of(asset: AssetId, who: &AccountId) -> Commitment;

    /// Get account's pending balance commitment
    fn pending_balance(asset: AssetId, who: &AccountId) -> Option<Commitment>;

    /// Get account's public key
    fn public_key(who: &AccountId) -> Option<PublicKeyBytes>;

    // === Public Key Management ===

    /// Register a public key for an account
    fn set_public_key(who: &AccountId, pk: &PublicKeyBytes) -> Result<(), Self::Error>;

    // === Balance Operations ===

    /// Execute a confidential transfer
    /// Returns the encrypted delta amount
    fn transfer_encrypted(
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        delta_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Claim pending transfers into available balance
    /// Returns the claimed encrypted amount (or zero marker)
    fn claim_encrypted(
        asset: AssetId,
        who: &AccountId,
        accept_envelope: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Mint new confidential balance (deposit path)
    fn mint_encrypted(
        asset: AssetId,
        to: &AccountId,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Burn confidential balance (withdraw path)
    /// Returns the disclosed plaintext amount
    fn burn_encrypted(
        asset: AssetId,
        from: &AccountId,
        amount_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<Balance, Self::Error>;

    /// Disclose an encrypted amount (owner-only)
    fn disclose_amount(
        asset: AssetId,
        cipher: &EncryptedAmount,
        who: &AccountId,
    ) -> Result<Balance, Self::Error>;
}
```

## Example: FHE Backend

A skeleton for an FHE-based backend:

```rust
use confidential_assets_primitives::*;

pub struct FheBackend;

impl<AccountId, AssetId, Balance> ConfidentialBackend<AccountId, AssetId, Balance>
    for FheBackend
where
    AccountId: Encode + Decode,
    AssetId: Encode + Decode + Copy,
    Balance: Into<u64> + From<u64>,
{
    type Error = FheError;

    fn total_supply(asset: AssetId) -> Commitment {
        // Read encrypted total from storage
        FheTotalSupply::<Runtime>::get(asset).unwrap_or_default()
    }

    fn balance_of(asset: AssetId, who: &AccountId) -> Commitment {
        FheBalances::<Runtime>::get(asset, who).unwrap_or_default()
    }

    fn set_public_key(who: &AccountId, pk: &PublicKeyBytes) -> Result<(), Self::Error> {
        // Store FHE public key (different format than ElGamal)
        FhePublicKeys::<Runtime>::insert(who, pk);
        Ok(())
    }

    fn transfer_encrypted(
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        delta_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error> {
        // With FHE, we can do computation on ciphertexts
        // No ZK proof needed - just homomorphic subtraction/addition

        let from_balance = FheBalances::<Runtime>::get(asset, from)
            .ok_or(FheError::NoBalance)?;
        let to_balance = FheBalances::<Runtime>::get(asset, to)
            .unwrap_or_default();

        // Homomorphic operations (pseudo-code)
        let new_from = fhe_subtract(&from_balance, &delta_ct)?;
        let new_to = fhe_add(&to_balance, &delta_ct)?;

        // Update storage
        FheBalances::<Runtime>::insert(asset, from, new_from);
        FheBalances::<Runtime>::insert(asset, to, new_to);

        Ok(delta_ct)
    }

    fn mint_encrypted(
        asset: AssetId,
        to: &AccountId,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error> {
        // Parse mint amount from proof (FHE-specific format)
        let (amount, ct) = parse_fhe_mint_proof(&proof)?;

        // Encrypt plaintext amount under recipient's key
        let recipient_pk = FhePublicKeys::<Runtime>::get(to)
            .ok_or(FheError::NoPk)?;
        let encrypted = fhe_encrypt(amount, &recipient_pk)?;

        // Homomorphically add to balance
        let current = FheBalances::<Runtime>::get(asset, to)
            .unwrap_or_default();
        let new_balance = fhe_add(&current, &encrypted)?;

        FheBalances::<Runtime>::insert(asset, to, new_balance);

        Ok(encrypted.into())
    }

    fn burn_encrypted(
        asset: AssetId,
        from: &AccountId,
        amount_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<Balance, Self::Error> {
        // Decrypt and verify burn amount (requires key holder interaction)
        let disclosed = fhe_decrypt_with_proof(&amount_ct, &proof)?;

        // Homomorphically subtract from balance
        let current = FheBalances::<Runtime>::get(asset, from)
            .ok_or(FheError::NoBalance)?;
        let new_balance = fhe_subtract(&current, &amount_ct)?;

        FheBalances::<Runtime>::insert(asset, from, new_balance);

        Ok(disclosed.into())
    }

    fn disclose_amount(
        asset: AssetId,
        cipher: &EncryptedAmount,
        who: &AccountId,
    ) -> Result<Balance, Self::Error> {
        // FHE disclosure typically requires interaction with key holder
        // This might be async in practice
        unimplemented!("FHE disclosure requires key holder")
    }

    // ... implement remaining methods
}
```

## Example: TEE Backend

For Trusted Execution Environment (e.g., Intel SGX):

```rust
pub struct TeeBackend;

impl<AccountId, AssetId, Balance> ConfidentialBackend<AccountId, AssetId, Balance>
    for TeeBackend
{
    type Error = TeeError;

    fn transfer_encrypted(
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        delta_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error> {
        // Verify TEE attestation
        let attestation = TeeAttestation::from_proof(&proof)?;
        verify_sgx_attestation(&attestation)?;

        // TEE has already verified the transfer internally
        // We just update the encrypted state it provided

        let new_states = attestation.output_states();
        TeeBalances::<Runtime>::insert(asset, from, new_states.from);
        TeeBalances::<Runtime>::insert(asset, to, new_states.to);

        Ok(delta_ct)
    }

    // ... other methods verify TEE attestations
}
```

## ZkVerifier Trait

If using ZK proofs, implement the verifier trait:

```rust
pub trait ZkVerifier {
    type Error;

    /// Verify sender transfer proof
    fn verify_transfer_sent(
        asset: &[u8],
        from_pk: &[u8],
        to_pk: &[u8],
        from_old_avail: &[u8],
        to_old_pending: &[u8],
        delta_ct: &[u8],
        proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error>;

    /// Verify receiver accept proof
    fn verify_transfer_received(
        asset: &[u8],
        who_pk: &[u8],
        avail_old: &[u8],
        pending_old: &[u8],
        commits: &[[u8; 32]],
        envelope: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error>;

    /// Verify mint proof
    fn verify_mint(
        asset: &[u8],
        to_pk: &PublicKeyBytes,
        to_old_pending: &[u8],
        total_old: &[u8],
        proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, EncryptedAmount), Self::Error>;

    /// Verify burn proof
    fn verify_burn(
        asset: &[u8],
        from_pk: &PublicKeyBytes,
        from_old_avail: &[u8],
        total_old: &[u8],
        amount_ct: &EncryptedAmount,
        proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, u64), Self::Error>;

    /// Disclose encrypted amount
    fn disclose(
        asset: &[u8],
        pk: &[u8],
        cipher: &[u8],
    ) -> Result<u64, Self::Error>;
}
```

## Registering Your Backend

Wire your backend into the runtime:

```rust
impl pallet_confidential_assets::Config for Runtime {
    type Backend = MyCustomBackend;  // Your implementation
    // ...
}
```

## Testing Custom Backends

Create comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_updates_balances() {
        new_test_ext().execute_with(|| {
            // Setup
            MyBackend::set_public_key(&ALICE, &alice_pk).unwrap();
            MyBackend::set_public_key(&BOB, &bob_pk).unwrap();

            // Mint to Alice
            MyBackend::mint_encrypted(ASSET, &ALICE, mint_proof).unwrap();

            // Transfer
            let result = MyBackend::transfer_encrypted(
                ASSET, &ALICE, &BOB, delta_ct, transfer_proof
            );
            assert!(result.is_ok());

            // Verify state changes
            // ...
        });
    }
}
```

## Performance Considerations

| Backend | Proof Size | Verification Time | Notes |
|---------|------------|-------------------|-------|
| ZK-ElGamal | ~1.2 KB | ~5 ms | Current default |
| Groth16 | ~200 B | ~3 ms | Requires trusted setup |
| FHE | N/A | ~100 ms | No proofs, but slow ops |
| TEE | ~1 KB | ~1 ms | Requires hardware |

## Migration Between Backends

To migrate from one backend to another:

1. Pause confidential operations
2. Decrypt all balances (requires key holders)
3. Re-encrypt under new scheme
4. Update runtime config
5. Resume operations

This is complex and should be avoided if possible.

## Next Steps

- [ACL & Operators](./acl-operators.md) - Access control options
- [Custom Ramps](./custom-ramps.md) - Custom deposit/withdraw logic
- [Testing Guide](./testing.md) - Test your backend
