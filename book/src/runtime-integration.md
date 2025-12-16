# Runtime Integration

Complete guide to integrating confidential assets into a Substrate runtime.

## Pallet Dependencies

Add these crates to your runtime's `Cargo.toml`:

```toml
[dependencies]
# Core pallets
pallet-zkhe = { path = "../pallets/zkhe", default-features = false }
pallet-confidential-assets = { path = "../pallets/confidential-assets", default-features = false }

# Optional: Cross-chain support
pallet-confidential-escrow = { path = "../pallets/confidential-escrow", default-features = false }
pallet-confidential-bridge = { path = "../pallets/confidential-bridge", default-features = false }

# On-chain verifier
zkhe-verifier = { path = "../zkhe/verifier", default-features = false }

# Shared primitives
confidential-assets-primitives = { path = "../primitives/confidential-assets", default-features = false }

[features]
std = [
    "pallet-zkhe/std",
    "pallet-confidential-assets/std",
    "pallet-confidential-escrow/std",
    "pallet-confidential-bridge/std",
    "zkhe-verifier/std",
    "confidential-assets-primitives/std",
]
```

## Type Definitions

Define your asset and balance types:

```rust
// In lib.rs or types.rs

/// Asset identifier - use same type as your assets pallet
pub type AssetId = u128;

/// Balance type - typically u128 for maximum range
pub type Balance = u128;

/// Account type - from your chain's signature scheme
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
```

## Pallet Configuration Module

Create a dedicated configuration file:

```rust
// In configs/confidential.rs

use crate::{AccountId, AssetId, Balance, Runtime, RuntimeEvent, RuntimeOrigin};
use confidential_assets_primitives::Ramp;
use frame_support::{parameter_types, traits::Get, PalletId};

// ==================== Network ID Provider ====================

/// Network ID provider for this runtime.
/// In production, use a unique identifier for your chain (e.g., genesis hash).
pub struct RuntimeNetworkId;
impl confidential_assets_primitives::NetworkIdProvider for RuntimeNetworkId {
    fn network_id() -> [u8; 32] {
        *b"my-chain-unique-identifier!" // Replace with your chain's unique ID
    }
}

// ==================== pallet-zkhe ====================

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier<RuntimeNetworkId>;
    type WeightInfo = ();  // Or use benchmarked weights
}

// ==================== pallet-confidential-assets ====================

impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = PublicRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}

// ==================== Ramp Implementation ====================

pub struct PublicRamp;

impl Ramp<AccountId, AssetId, Balance> for PublicRamp {
    type Error = sp_runtime::DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        // Implement based on your assets pallet
        pallet_assets::Pallet::<Runtime>::transfer(
            RuntimeOrigin::signed(from.clone()),
            asset.into(),
            to.clone().into(),
            amount,
        )?;
        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        pallet_assets::Pallet::<Runtime>::mint(
            RuntimeOrigin::root(),  // Or use privileged origin
            (*asset).into(),
            to.clone().into(),
            amount,
        )?;
        Ok(())
    }

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        pallet_assets::Pallet::<Runtime>::burn(
            RuntimeOrigin::signed(from.clone()),
            (*asset).into(),
            from.clone().into(),
            amount,
        )?;
        Ok(())
    }
}
```

## construct_runtime! Integration

Add pallets to your runtime:

```rust
// In lib.rs

#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        // ... other derives
    )]
    pub struct Runtime;

    // System pallets (indices 0-9)
    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    // ... other system pallets

    // Monetary pallets (indices 10-19)
    #[runtime::pallet_index(10)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(11)]
    pub type Assets = pallet_assets;
    #[runtime::pallet_index(12)]
    pub type TransactionPayment = pallet_transaction_payment;

    // XCM pallets (indices 30-39)
    #[runtime::pallet_index(30)]
    pub type XcmpQueue = cumulus_pallet_xcmp_queue;
    #[runtime::pallet_index(31)]
    pub type PolkadotXcm = pallet_xcm;
    // ... other XCM pallets

    // Confidential pallets (indices 40-49)
    #[runtime::pallet_index(40)]
    pub type Zkhe = pallet_zkhe;
    #[runtime::pallet_index(41)]
    pub type ConfidentialAssets = pallet_confidential_assets;
    #[runtime::pallet_index(42)]
    pub type ConfidentialEscrow = pallet_confidential_escrow;
    #[runtime::pallet_index(43)]
    pub type ConfidentialBridge = pallet_confidential_bridge;
}
```

## Genesis Configuration

Configure initial state if needed:

```rust
// In genesis_config_presets.rs

fn development_genesis_config() -> serde_json::Value {
    serde_json::json!({
        // Zkhe has no genesis config by default
        "zkhe": {},

        // ConfidentialAssets has no genesis config by default
        "confidentialAssets": {},

        // Other pallets...
    })
}
```

## Runtime APIs

Expose confidential assets via runtime APIs:

```rust
// In apis.rs

impl_runtime_apis! {
    // ... other APIs

    impl confidential_assets_primitives::ConfidentialAssetsApi<Block, AssetId, AccountId, Balance>
        for Runtime
    {
        fn total_supply(asset: AssetId) -> [u8; 32] {
            ConfidentialAssets::confidential_total_supply(asset)
        }

        fn balance_of(asset: AssetId, who: AccountId) -> [u8; 32] {
            ConfidentialAssets::confidential_balance_of(asset, &who)
        }

        fn public_key(who: AccountId) -> Option<Vec<u8>> {
            Zkhe::public_key(&who).map(|pk| pk.to_vec())
        }

        fn pending_balance(asset: AssetId, who: AccountId) -> Option<[u8; 32]> {
            pallet_zkhe::PendingBalanceCommit::<Runtime>::get(asset, who)
        }
    }
}
```

## Module Configuration

Ensure the configuration module is included:

```rust
// In lib.rs

pub mod configs;
pub use configs::*;
```

And in your configs module:

```rust
// In configs/mod.rs

mod confidential;
pub use confidential::*;

// ... other config modules
```

## Migrations

Handle storage migrations for upgrades:

```rust
// In lib.rs

type Migrations = (
    // Add migrations here when upgrading
    // pallet_zkhe::migrations::v1::MigrateToV1<Runtime>,
);

pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;
```

## Testing the Integration

Add integration tests:

```rust
// In tests/integration.rs

#[test]
fn confidential_transfer_works() {
    new_test_ext().execute_with(|| {
        // Setup public keys
        assert_ok!(ConfidentialAssets::set_public_key(
            RuntimeOrigin::signed(ALICE),
            alice_pk.to_vec().try_into().unwrap()
        ));

        // Deposit to confidential
        assert_ok!(ConfidentialAssets::deposit(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            1000,
            mint_proof
        ));

        // Transfer confidentially
        assert_ok!(ConfidentialAssets::confidential_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            BOB,
            delta_ct,
            sender_proof
        ));
    });
}
```

## Build Verification

Verify the runtime builds correctly:

```bash
# Build runtime
cargo build --release -p your-runtime

# Run tests
cargo test -p your-runtime --all-features

# Check WASM size
ls -la target/release/wbuild/your-runtime/*.wasm
```

## Troubleshooting

### Common Issues

**"trait bound not satisfied" errors:**
```rust
// Ensure all associated types implement required traits
type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
```

**WASM build fails:**
```bash
# Ensure no_std compatibility
cargo build --target wasm32-unknown-unknown -p your-runtime
```

**Missing features:**
```toml
[features]
std = [
    # Don't forget any dependencies
    "zkhe-verifier/std",
]
```

## Next Steps

- [XCM Setup](./xcm-setup.md) - Configure cross-chain transfers
- [Custom Backends](./custom-backends.md) - Use alternative backends
- [Testing Guide](./testing.md) - Comprehensive testing
