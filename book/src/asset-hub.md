# Asset Hub Integration

This guide explains how to integrate confidential assets with **Polkadot Asset Hub** for production deployment.

## Overview

Asset Hub is Polkadot's system parachain for managing fungible and non-fungible assets. Integrating confidential assets enables:

- **Confidential DOT transfers** - Transfer DOT with hidden amounts
- **Confidential stablecoin transfers** - USDT, USDC with privacy
- **Privacy-preserving DeFi** - Private swaps, lending, etc.
- **Cross-chain confidential transfers** - XCM between parachains

## Integration Architecture

```text
┌───────────────────────────────────────────────────────────────┐
│                      Asset Hub Runtime                        │
│                                                               │
│  ┌─────────────────┐  ┌─────────────────┐                     │
│  │  pallet-assets  │  │ pallet-balances │                     │
│  │ (USDT, USDC...) │  │      (DOT)      │                     │
│  └────────┬────────┘  └────────┬────────┘                     │
│           │                    │                              │
│           └────────┬───────────┘                              │
│                    │ Ramp trait                               │
│           ┌────────▼────────┐                                 │
│           │   PublicRamp    │                                 │
│           │   (burn/mint)   │                                 │
│           └────────┬────────┘                                 │
│                    │                                          │
│  ┌─────────────────▼───────────────────────────────────────┐  │
│  │            pallet-confidential-assets                   │  │
│  │  ┌───────────────────────────────────────────────────┐  │  │
│  │  │                  pallet-zkhe                      │  │  │
│  │  │  ┌─────────────────────────────────────────────┐  │  │  │
│  │  │  │          zkhe-verifier (no_std)             │  │  │  │
│  │  │  └─────────────────────────────────────────────┘  │  │  │
│  │  └───────────────────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

## Step 1: Add Pallets to Asset Hub Runtime

```rust
// In asset-hub-polkadot-runtime/Cargo.toml
[dependencies]
pallet-zkhe = { git = "...", default-features = false }
pallet-confidential-assets = { git = "...", default-features = false }
zkhe-verifier = { git = "...", default-features = false }
confidential-assets-primitives = { git = "...", default-features = false }
```

## Step 2: Configure Asset Types

Asset Hub uses `u32` for asset IDs and `u128` for balances:

```rust
// In asset-hub-polkadot-runtime/src/lib.rs

/// Asset Hub uses u32 asset IDs
pub type AssetIdForConfidential = u32;

/// Native asset (DOT) identifier
pub const NATIVE_ASSET_ID: AssetIdForConfidential = 0;

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetIdForConfidential;
    type Balance = Balance;  // u128
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = weights::pallet_zkhe::WeightInfo<Runtime>;
}

impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetIdForConfidential;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = AssetHubRamp;
    type AssetMetadata = AssetHubMetadata;
    type Acl = ();
    type Operators = ();
    type WeightInfo = weights::pallet_confidential_assets::WeightInfo<Runtime>;
}
```

## Step 3: Implement Asset Hub Ramp

The ramp must handle both DOT (native) and pallet-assets tokens:

```rust
use pallet_assets::Pallet as Assets;
use pallet_balances::Pallet as Balances;

pub struct AssetHubRamp;

impl Ramp<AccountId, AssetIdForConfidential, Balance> for AssetHubRamp {
    type Error = DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetIdForConfidential,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        if asset == NATIVE_ASSET_ID {
            // DOT: Use pallet-balances
            <Balances as Currency<AccountId>>::transfer(
                from,
                to,
                amount,
                ExistenceRequirement::AllowDeath,
            )
        } else {
            // Other assets: Use pallet-assets
            <Assets as Mutate<AccountId>>::transfer(
                asset,
                from,
                to,
                amount,
                Preservation::Expendable,
            )
        }
    }

    fn mint(to: &AccountId, asset: &AssetIdForConfidential, amount: Balance) -> Result<(), Self::Error> {
        if *asset == NATIVE_ASSET_ID {
            // DOT: Create imbalance (requires careful handling)
            let _ = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
            Ok(())
        } else {
            // Other assets: Mint via pallet-assets
            <Assets as Mutate<AccountId>>::mint_into(*asset, to, amount)
        }
    }

    fn burn(from: &AccountId, asset: &AssetIdForConfidential, amount: Balance) -> Result<(), Self::Error> {
        if *asset == NATIVE_ASSET_ID {
            // DOT: Withdraw and drop imbalance
            let _ = <Balances as Currency<AccountId>>::withdraw(
                from,
                amount,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok(())
        } else {
            // Other assets: Burn via pallet-assets
            <Assets as Mutate<AccountId>>::burn_from(
                *asset,
                from,
                amount,
                Preservation::Expendable,
                Precision::BestEffort,
                Fortitude::Polite,
            )?;
            Ok(())
        }
    }
}
```

## Step 4: Asset Metadata Provider

Expose Asset Hub's metadata for confidential assets:

```rust
use confidential_assets_primitives::AssetMetadataProvider;

pub struct AssetHubMetadata;

impl AssetMetadataProvider<AssetIdForConfidential> for AssetHubMetadata {
    fn name(asset: AssetIdForConfidential) -> Vec<u8> {
        if asset == NATIVE_ASSET_ID {
            b"Polkadot".to_vec()
        } else {
            Assets::name(asset).into_inner()
        }
    }

    fn symbol(asset: AssetIdForConfidential) -> Vec<u8> {
        if asset == NATIVE_ASSET_ID {
            b"DOT".to_vec()
        } else {
            Assets::symbol(asset).into_inner()
        }
    }

    fn decimals(asset: AssetIdForConfidential) -> u8 {
        if asset == NATIVE_ASSET_ID {
            10  // DOT has 10 decimals
        } else {
            Assets::decimals(asset)
        }
    }
}
```

## Step 5: Add to construct_runtime!

```rust
construct_runtime!(
    pub enum Runtime {
        // ... existing Asset Hub pallets ...

        // Confidential Assets (indices should be coordinated)
        Zkhe: pallet_zkhe = 60,
        ConfidentialAssets: pallet_confidential_assets = 61,

        // Optional: For cross-chain
        ConfidentialEscrow: pallet_confidential_escrow = 62,
        ConfidentialBridge: pallet_confidential_bridge = 63,
    }
);
```

## Step 6: XCM Configuration (Optional)

For cross-chain confidential transfers from Asset Hub:

```rust
// In xcm_config.rs

parameter_types! {
    pub const MaxBridgePayload: u32 = 16 * 1024;
}

impl pallet_confidential_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetIdForConfidential;
    type Balance = Balance;
    type Backend = Zkhe;
    type Escrow = ConfidentialEscrow;
    type Messenger = XcmHrmpMessenger;
    type MaxBridgePayload = MaxBridgePayload;
    type BurnPalletId = ConfidentialBridgePalletId;
    type DefaultTimeout = ConstU32<100>;
    type SelfParaId = ParachainInfo;
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type WeightInfo = ();
}
```

## Supported Assets

| Asset | Asset ID | Notes |
|-------|----------|-------|
| DOT | 0 | Native token, uses pallet-balances |
| USDT | 1984 | Tether USD on Asset Hub |
| USDC | 1337 | Circle USD on Asset Hub |
| Custom | varies | Any pallet-assets token |

## User Flow: Confidential DOT Transfer

```text
1. Alice registers her public key
   ConfidentialAssets::set_public_key(alice_pk)

2. Alice deposits 100 DOT into confidential balance
   ConfidentialAssets::deposit(
       asset_id: 0,      // DOT
       amount: 100_000_000_000,  // 100 DOT (10 decimals)
       proof: mint_proof
   )

3. Alice transfers 50 DOT confidentially to Bob
   ConfidentialAssets::confidential_transfer(
       asset_id: 0,
       to: bob_address,
       encrypted_amount: delta_ct,
       proof: sender_bundle
   )

4. Bob claims his pending transfer
   ConfidentialAssets::confidential_claim(
       asset_id: 0,
       accept_envelope: accept_proof
   )

5. Bob withdraws 50 DOT back to public balance
   ConfidentialAssets::withdraw(
       asset_id: 0,
       encrypted_amount: ct,
       proof: burn_proof
   )
```

## Security Considerations

### Privileged Operations

Carefully consider which accounts can:
- Mint new confidential assets (deposit path)
- Burn confidential assets (withdraw path)
- Act as cross-chain bridge operators

### Weight/Fee Estimation

ZK proof verification is computationally expensive:

```rust
// Approximate weights (adjust based on benchmarks)
impl WeightData for AssetHubWeights {
    fn confidential_transfer() -> Weight {
        Weight::from_parts(500_000_000, 10_000)  // ~500ms, 10KB PoV
    }
}
```

### Rate Limiting

Consider implementing rate limits for:
- Maximum deposits per block
- Maximum transfers per account per block
- Cross-chain transfer cooldowns

## Testing on Westend Asset Hub

Before mainnet deployment, test on Westend Asset Hub:

```bash
# Run local Westend Asset Hub with confidential pallets
./target/release/polkadot-parachain \
    --chain westend-asset-hub-local \
    --alice \
    --tmp
```

## Next Steps

- [XCM Setup](./xcm-setup.md) - Configure cross-chain transfers
- [Configuration Guide](./configuration.md) - Fine-tune parameters
- [Client Integration](./client.md) - Build user-facing apps
