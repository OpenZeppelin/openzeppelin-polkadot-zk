# API Reference

Complete API documentation for all confidential assets pallets and traits.

## pallet-confidential-assets

The main pallet for confidential asset operations.

### Extrinsics

#### `set_public_key`

Register an ElGamal public key for confidential operations.

```rust
pub fn set_public_key(
    origin: OriginFor<T>,
    pk: BoundedVec<u8, ConstU32<32>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (account registering the key)
- `pk`: 32-byte ElGamal public key

**Errors:**
- `PkAlreadySet`: Account already has a registered public key

**Events:**
- `PublicKeySet { who: AccountId }`

---

#### `deposit`

Deposit public assets into confidential balance.

```rust
pub fn deposit(
    origin: OriginFor<T>,
    asset: T::AssetId,
    amount: T::Balance,
    proof: BoundedVec<u8, ConstU32<2048>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (depositor)
- `asset`: Asset identifier
- `amount`: Amount to deposit (plaintext)
- `proof`: ZK mint proof

**Errors:**
- `NoPk`: Depositor has no registered public key
- `ProofVerificationFailed`: Invalid mint proof

**Events:**
- `Deposit { asset: AssetId, who: AccountId, amount: Balance }`

---

#### `confidential_transfer`

Transfer confidential assets to another account.

```rust
pub fn confidential_transfer(
    origin: OriginFor<T>,
    asset: T::AssetId,
    to: T::AccountId,
    delta_ct: EncryptedAmount,
    proof: BoundedVec<u8, ConstU32<2048>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (sender)
- `asset`: Asset identifier
- `to`: Recipient account
- `delta_ct`: Encrypted transfer amount
- `proof`: ZK sender transfer proof

**Errors:**
- `NoPk`: Sender or recipient missing public key
- `ProofVerificationFailed`: Invalid transfer proof
- `AclRejected`: Transfer blocked by ACL

**Events:**
- `ConfidentialTransfer { asset: AssetId, from: AccountId, to: AccountId, encrypted_amount: EncryptedAmount }`

---

#### `confidential_transfer_from`

Transfer on behalf of another account (requires operator approval).

```rust
pub fn confidential_transfer_from(
    origin: OriginFor<T>,
    asset: T::AssetId,
    from: T::AccountId,
    to: T::AccountId,
    delta_ct: EncryptedAmount,
    proof: BoundedVec<u8, ConstU32<2048>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (operator)
- `asset`: Asset identifier
- `from`: Owner account
- `to`: Recipient account
- `delta_ct`: Encrypted transfer amount
- `proof`: ZK sender transfer proof

**Errors:**
- `NotAuthorized`: Caller is not an approved operator
- All errors from `confidential_transfer`

---

#### `accept_pending`

Claim pending transfers into available balance.

```rust
pub fn accept_pending(
    origin: OriginFor<T>,
    asset: T::AssetId,
    proof: BoundedVec<u8, ConstU32<2048>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (recipient)
- `asset`: Asset identifier
- `proof`: ZK receiver accept proof (envelope)

**Errors:**
- `NoPk`: Account has no registered public key
- `PendingNotFound`: No pending balance to claim
- `ProofVerificationFailed`: Invalid accept proof

**Events:**
- `PendingAccepted { asset: AssetId, who: AccountId, encrypted_amount: EncryptedAmount }`

---

#### `withdraw`

Withdraw confidential assets to public balance.

```rust
pub fn withdraw(
    origin: OriginFor<T>,
    asset: T::AssetId,
    amount_ct: EncryptedAmount,
    proof: BoundedVec<u8, ConstU32<2048>>,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (withdrawer)
- `asset`: Asset identifier
- `amount_ct`: Encrypted amount to withdraw
- `proof`: ZK burn proof

**Errors:**
- `NoPk`: Account has no registered public key
- `ProofVerificationFailed`: Invalid burn proof

**Events:**
- `Withdrawal { asset: AssetId, who: AccountId, amount: Balance }`

---

#### `disclose`

Disclose an encrypted amount (owner only).

```rust
pub fn disclose(
    origin: OriginFor<T>,
    asset: T::AssetId,
    cipher: EncryptedAmount,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (owner of the encrypted amount)
- `asset`: Asset identifier
- `cipher`: Encrypted amount to disclose

**Events:**
- `Disclosed { asset: AssetId, who: AccountId, amount: Balance }`

---

#### `transfer_and_call`

Transfer with additional data for contract calls.

```rust
pub fn transfer_and_call(
    origin: OriginFor<T>,
    asset: T::AssetId,
    to: T::AccountId,
    delta_ct: EncryptedAmount,
    proof: BoundedVec<u8, ConstU32<2048>>,
    data: BoundedVec<u8, ConstU32<1024>>,
) -> DispatchResult
```

### Storage

#### `confidential_total_supply`

Total supply commitment for an asset.

```rust
fn confidential_total_supply(asset: AssetId) -> [u8; 32]
```

#### `confidential_balance_of`

Available balance commitment for an account.

```rust
fn confidential_balance_of(asset: AssetId, who: &AccountId) -> [u8; 32]
```

---

## pallet-zkhe

The ZK-ElGamal backend pallet.

### Storage Items

#### `PublicKeys`

Mapping of accounts to ElGamal public keys.

```rust
StorageMap<_, Blake2_128Concat, AccountId, PublicKeyBytes>
```

#### `AvailableBalanceCommit`

Available balance commitments.

```rust
StorageDoubleMap<_, Twox64Concat, AssetId, Blake2_128Concat, AccountId, Commitment>
```

#### `PendingBalanceCommit`

Pending balance commitments.

```rust
StorageDoubleMap<_, Twox64Concat, AssetId, Blake2_128Concat, AccountId, Commitment>
```

#### `TotalSupplyCommit`

Total supply commitments per asset.

```rust
StorageMap<_, Twox64Concat, AssetId, Commitment>
```

#### `PendingUtxos`

Pending UTXOs for an account.

```rust
StorageDoubleMap<_, Twox64Concat, AssetId, Blake2_128Concat, AccountId, BoundedVec<UtxoId, MaxUtxos>>
```

#### `UtxoCommits`

UTXO commitments by ID.

```rust
StorageMap<_, Twox64Concat, UtxoId, Commitment>
```

#### `NextUtxoId`

Counter for UTXO IDs.

```rust
StorageValue<_, UtxoId>
```

---

## Traits

### `ConfidentialBackend`

Core trait for confidential balance operations.

```rust
pub trait ConfidentialBackend<AccountId, AssetId, Balance> {
    type Error;

    /// Get total supply commitment
    fn total_supply(asset: AssetId) -> Commitment;

    /// Get available balance commitment
    fn balance_of(asset: AssetId, who: &AccountId) -> Commitment;

    /// Get pending balance commitment
    fn pending_balance(asset: AssetId, who: &AccountId) -> Option<Commitment>;

    /// Get account's public key
    fn public_key(who: &AccountId) -> Option<PublicKeyBytes>;

    /// Register public key
    fn set_public_key(who: &AccountId, pk: &PublicKeyBytes) -> Result<(), Self::Error>;

    /// Execute confidential transfer
    fn transfer_encrypted(
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        delta_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Claim pending balance
    fn claim_encrypted(
        asset: AssetId,
        who: &AccountId,
        accept_envelope: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Mint confidential balance
    fn mint_encrypted(
        asset: AssetId,
        to: &AccountId,
        proof: InputProof,
    ) -> Result<EncryptedAmount, Self::Error>;

    /// Burn confidential balance
    fn burn_encrypted(
        asset: AssetId,
        from: &AccountId,
        amount_ct: EncryptedAmount,
        proof: InputProof,
    ) -> Result<Balance, Self::Error>;

    /// Disclose encrypted amount
    fn disclose_amount(
        asset: AssetId,
        cipher: &EncryptedAmount,
        who: &AccountId,
    ) -> Result<Balance, Self::Error>;
}
```

### `ZkVerifier`

ZK proof verification trait.

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

### `Ramp`

Public/confidential asset bridge trait.

```rust
pub trait Ramp<AccountId, AssetId, Balance> {
    type Error: Into<DispatchError>;

    /// Transfer public assets
    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;

    /// Mint public assets (for withdrawals)
    fn mint(
        to: &AccountId,
        asset: &AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;

    /// Burn public assets (for deposits)
    fn burn(
        from: &AccountId,
        asset: &AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;
}
```

### `AclProvider`

Access control trait.

```rust
pub trait AclProvider<AccountId, AssetId, Balance> {
    /// Check if transfer is authorized
    fn authorized(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        encrypted_amount: &EncryptedAmount,
    ) -> bool;

    /// Check if transfer_and_call is authorized
    fn authorized_for_call(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        encrypted_amount: &EncryptedAmount,
        data: &[u8],
    ) -> bool {
        Self::authorized(caller, asset, from, to, encrypted_amount)
    }
}
```

### `OperatorRegistry`

Operator delegation trait.

```rust
pub trait OperatorRegistry<AccountId, AssetId, BlockNumber> {
    /// Check if operator is authorized
    fn is_operator(
        owner: &AccountId,
        operator: &AccountId,
        asset: AssetId,
    ) -> bool;

    /// Set operator approval
    fn set_approval(
        owner: &AccountId,
        operator: &AccountId,
        asset: AssetId,
        approved: bool,
        expiry: Option<BlockNumber>,
    ) -> DispatchResult;
}
```

---

## Types

### `EncryptedAmount`

64-byte encrypted amount (Twisted ElGamal ciphertext).

```rust
pub type EncryptedAmount = [u8; 64];
```

### `Commitment`

32-byte Pedersen commitment.

```rust
pub type Commitment = [u8; 32];
```

### `PublicKeyBytes`

32-byte ElGamal public key.

```rust
pub type PublicKeyBytes = [u8; 32];
```

### `UtxoId`

UTXO identifier.

```rust
pub type UtxoId = u64;
```

### `InputProof`

Bounded proof bytes.

```rust
pub type InputProof = BoundedVec<u8, ConstU32<2048>>;
```

---

## Runtime APIs

### `ConfidentialAssetsApi`

Runtime API for querying confidential state.

```rust
sp_api::decl_runtime_apis! {
    pub trait ConfidentialAssetsApi<AssetId, AccountId, Balance> {
        /// Get total supply commitment
        fn total_supply(asset: AssetId) -> [u8; 32];

        /// Get balance commitment
        fn balance_of(asset: AssetId, who: AccountId) -> [u8; 32];

        /// Get public key
        fn public_key(who: AccountId) -> Option<Vec<u8>>;

        /// Get pending balance commitment
        fn pending_balance(asset: AssetId, who: AccountId) -> Option<[u8; 32]>;
    }
}
```

---

## Events

### pallet-confidential-assets

```rust
pub enum Event<T: Config> {
    /// Public key registered
    PublicKeySet { who: T::AccountId },

    /// Assets deposited to confidential
    Deposit {
        asset: T::AssetId,
        who: T::AccountId,
        amount: T::Balance,
    },

    /// Confidential transfer executed
    ConfidentialTransfer {
        asset: T::AssetId,
        from: T::AccountId,
        to: T::AccountId,
        encrypted_amount: EncryptedAmount,
    },

    /// Pending balance accepted
    PendingAccepted {
        asset: T::AssetId,
        who: T::AccountId,
        encrypted_amount: EncryptedAmount,
    },

    /// Assets withdrawn from confidential
    Withdrawal {
        asset: T::AssetId,
        who: T::AccountId,
        amount: T::Balance,
    },

    /// Amount disclosed
    Disclosed {
        asset: T::AssetId,
        who: T::AccountId,
        amount: T::Balance,
    },
}
```

---

## Errors

### pallet-confidential-assets

```rust
pub enum Error<T> {
    /// Public key already registered
    PkAlreadySet,

    /// Caller not authorized
    NotAuthorized,

    /// Transfer blocked by ACL
    AclRejected,
}
```

### pallet-zkhe

```rust
pub enum Error<T> {
    /// No public key registered
    NoPk,

    /// Public key already set
    PkAlreadySet,

    /// ZK proof verification failed
    ProofVerificationFailed,

    /// No pending balance
    PendingNotFound,

    /// Too many pending UTXOs
    TooManyUtxos,
}
```

---

## Constants

### Weight Constants

```rust
// Approximate weights (benchmark for accurate values)
pub const SET_PUBLIC_KEY_WEIGHT: Weight = Weight::from_parts(10_000_000, 0);
pub const DEPOSIT_WEIGHT: Weight = Weight::from_parts(50_000_000, 0);
pub const TRANSFER_WEIGHT: Weight = Weight::from_parts(100_000_000, 0);
pub const ACCEPT_PENDING_WEIGHT: Weight = Weight::from_parts(80_000_000, 0);
pub const WITHDRAW_WEIGHT: Weight = Weight::from_parts(60_000_000, 0);
```

### Proof Size Limits

```rust
pub const MAX_PROOF_SIZE: u32 = 2048;  // bytes
pub const MAX_CALL_DATA_SIZE: u32 = 1024;  // bytes
pub const MAX_BRIDGE_PAYLOAD: u32 = 16384;  // bytes (16 KiB)
```

### UTXO Limits

```rust
pub const MAX_PENDING_UTXOS: u32 = 256;
```

---

## Next Steps

- [Architecture](./architecture.md) - System design overview
- [Testing Guide](./testing.md) - Test your integration
- [Client Integration](./client.md) - Build client applications
