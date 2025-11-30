# ACL & Operators

Configure access control and operator permissions for confidential transfers.

## Overview

The ERC-7984 standard supports two permission layers:

1. **ACL (Access Control List)**: Protocol-level rules for transfer authorization
2. **Operators**: Account-level delegation of transfer rights

## ACL Provider

The ACL controls which transfers are allowed at the protocol level.

### AclProvider Trait

```rust
pub trait AclProvider<AccountId, AssetId, Balance> {
    /// Check if a confidential transfer is authorized
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
        // Default: same as regular transfer
        Self::authorized(caller, asset, from, to, encrypted_amount)
    }
}
```

### Default Implementation (Allow All)

```rust
// Using () allows all transfers
impl<A, I, B> AclProvider<A, I, B> for () {
    fn authorized(_: &A, _: I, _: &A, _: &A, _: &EncryptedAmount) -> bool {
        true
    }
}

// In runtime config:
impl pallet_confidential_assets::Config for Runtime {
    type Acl = ();  // Allow all transfers
}
```

### Custom ACL Example

```rust
pub struct AssetWhitelistAcl;

impl AclProvider<AccountId, AssetId, Balance> for AssetWhitelistAcl {
    fn authorized(
        _caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        _encrypted_amount: &EncryptedAmount,
    ) -> bool {
        // Only allow transfers of whitelisted assets
        if !WhitelistedAssets::get().contains(&asset) {
            return false;
        }

        // Only allow transfers between KYC'd accounts
        if !KycRegistry::is_verified(from) || !KycRegistry::is_verified(to) {
            return false;
        }

        true
    }
}
```

### Compliance ACL

For regulated environments:

```rust
pub struct ComplianceAcl;

impl AclProvider<AccountId, AssetId, Balance> for ComplianceAcl {
    fn authorized(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        _encrypted_amount: &EncryptedAmount,
    ) -> bool {
        // Check sanctions lists
        if Sanctions::is_sanctioned(from) || Sanctions::is_sanctioned(to) {
            return false;
        }

        // Check transfer limits (daily, per-tx)
        if !TransferLimits::within_limits(from, asset) {
            return false;
        }

        // Check jurisdiction rules
        if !JurisdictionRules::transfer_allowed(from, to, asset) {
            return false;
        }

        true
    }

    fn authorized_for_call(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        encrypted_amount: &EncryptedAmount,
        data: &[u8],
    ) -> bool {
        // Additional checks for contract calls
        if !Self::authorized(caller, asset, from, to, encrypted_amount) {
            return false;
        }

        // Verify target contract is approved
        let target = decode_call_target(data);
        ApprovedContracts::contains(&target)
    }
}
```

## Operator Registry

Operators allow account owners to delegate transfer rights.

### OperatorRegistry Trait

```rust
pub trait OperatorRegistry<AccountId, AssetId, BlockNumber> {
    /// Check if an operator is authorized for an account
    fn is_operator(
        owner: &AccountId,
        operator: &AccountId,
        asset: AssetId,
    ) -> bool;

    /// Set operator approval (called by owner)
    fn set_approval(
        owner: &AccountId,
        operator: &AccountId,
        asset: AssetId,
        approved: bool,
        expiry: Option<BlockNumber>,
    ) -> DispatchResult;
}
```

### Default Implementation (No Operators)

```rust
// Using () means operators are disabled
impl<A, I, B> OperatorRegistry<A, I, B> for () {
    fn is_operator(_: &A, _: &A, _: I) -> bool {
        false  // No delegated transfers
    }

    fn set_approval(_: &A, _: &A, _: I, _: bool, _: Option<B>) -> DispatchResult {
        Err(DispatchError::Other("Operators not supported"))
    }
}
```

### pallet-operators

A full operator implementation:

```rust
// Add to runtime
impl pallet_operators::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type MaxOperatorsPerAccount = ConstU32<10>;
}

// Use in confidential-assets
impl pallet_confidential_assets::Config for Runtime {
    type Operators = Operators;  // pallet-operators
}
```

### Using pallet-operators

```rust
// Owner approves operator
Operators::set_approval_for_all(
    RuntimeOrigin::signed(owner),
    operator,
    asset_id,
    true,  // approved
    Some(current_block + 1000),  // expiry
)?;

// Operator can now transfer on behalf of owner
ConfidentialAssets::confidential_transfer_from(
    RuntimeOrigin::signed(operator),  // Operator signs
    asset_id,
    owner,      // From owner's account
    recipient,
    delta_ct,
    proof,
)?;
```

## Transfer Flow with ACL & Operators

```
┌─────────────────────────────────────────────────────────────┐
│                    Transfer Request                          │
│  caller, asset, from, to, encrypted_amount, proof           │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Authorization Check                         │
│                                                              │
│  1. Is caller == from?                                      │
│     YES → Proceed to ACL check                              │
│     NO  → Check operator status                             │
│                                                              │
│  2. If caller != from:                                       │
│     Operators::is_operator(from, caller, asset)?            │
│     NO  → Error: NotAuthorized                              │
│     YES → Proceed to ACL check                              │
│                                                              │
│  3. ACL check:                                               │
│     Acl::authorized(caller, asset, from, to, amount)?       │
│     NO  → Error: AclRejected                                │
│     YES → Execute transfer                                  │
└─────────────────────────────────────────────────────────────┘
```

## Combined ACL + Operators Example

```rust
pub struct RegulatedTransfers;

impl AclProvider<AccountId, AssetId, Balance> for RegulatedTransfers {
    fn authorized(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        _encrypted_amount: &EncryptedAmount,
    ) -> bool {
        // Allow transfers between verified accounts
        KycRegistry::is_verified(from) && KycRegistry::is_verified(to)
    }
}

// Combined with operators:
impl pallet_confidential_assets::Config for Runtime {
    type Acl = RegulatedTransfers;
    type Operators = Operators;
}

// Now:
// - Only KYC'd accounts can send/receive
// - Account owners can delegate to operators
// - Both checks must pass for transfer_from
```

## Per-Asset ACL

Different rules per asset:

```rust
pub struct PerAssetAcl;

impl AclProvider<AccountId, AssetId, Balance> for PerAssetAcl {
    fn authorized(
        caller: &AccountId,
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        encrypted_amount: &EncryptedAmount,
    ) -> bool {
        match asset {
            // DOT: unrestricted
            0 => true,

            // USDT: KYC required
            1984 => KycRegistry::is_verified(from) && KycRegistry::is_verified(to),

            // Internal token: whitelist only
            9999 => InternalWhitelist::contains(from) && InternalWhitelist::contains(to),

            // Unknown assets: blocked
            _ => false,
        }
    }
}
```

## Events

Track authorization events:

```rust
// Operator events (from pallet-operators)
Event::ApprovalSet { owner, operator, asset, approved, expiry }
Event::ApprovalRevoked { owner, operator, asset }

// Transfer events include caller info
Event::ConfidentialTransfer { asset, from, to, encrypted_amount }
// Note: from may differ from extrinsic signer if operator
```

## Storage

Operator storage (in pallet-operators):

```rust
#[pallet::storage]
pub type Approvals<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat, T::AccountId,  // owner
    Blake2_128Concat, (T::AccountId, T::AssetId),  // (operator, asset)
    ApprovalInfo<BlockNumberFor<T>>,
>;

pub struct ApprovalInfo<BlockNumber> {
    pub approved: bool,
    pub expiry: Option<BlockNumber>,
}
```

## Security Considerations

### Operator Risks

1. **Unlimited delegation**: Consider per-operator transfer limits
2. **Expiry**: Always set expiry for operator approvals
3. **Revocation**: Provide easy way to revoke all operators

### ACL Bypass

1. **Direct backend calls**: ACL only applies to pallet extrinsics
2. **Cross-chain**: XCM transfers may bypass local ACL
3. **Upgrades**: ACL changes apply to future transfers only

## Testing

```rust
#[test]
fn acl_blocks_unauthorized_transfers() {
    new_test_ext().execute_with(|| {
        // Setup: Bob is not KYC'd
        KycRegistry::set_verified(&ALICE, true);
        KycRegistry::set_verified(&BOB, false);

        // Alice tries to transfer to Bob
        let result = ConfidentialAssets::confidential_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            BOB,
            ct,
            proof,
        );

        assert_noop!(result, Error::<Runtime>::AclRejected);
    });
}

#[test]
fn operator_can_transfer_for_owner() {
    new_test_ext().execute_with(|| {
        // Setup
        Operators::set_approval_for_all(
            RuntimeOrigin::signed(ALICE),
            BOB,  // operator
            ASSET,
            true,
            None,
        ).unwrap();

        // Bob transfers on Alice's behalf
        let result = ConfidentialAssets::confidential_transfer_from(
            RuntimeOrigin::signed(BOB),
            ASSET,
            ALICE,
            CHARLIE,
            ct,
            proof,
        );

        assert_ok!(result);
    });
}
```

## Next Steps

- [Custom Ramps](./custom-ramps.md) - Custom deposit/withdraw logic
- [Testing Guide](./testing.md) - Test your ACL implementation
- [Client Integration](./client.md) - Handle ACL in your UI
