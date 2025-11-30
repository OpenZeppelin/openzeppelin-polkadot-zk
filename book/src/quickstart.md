# Quick Start

Get confidential assets running on your parachain in 5 minutes.

## Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- A Substrate parachain project (or use the included runtime template)

## Step 1: Add Dependencies

Add to your runtime's `Cargo.toml`:

```toml
[dependencies]
# Core pallets
pallet-zkhe = { path = "path/to/pallets/zkhe", default-features = false }
pallet-confidential-assets = { path = "path/to/pallets/confidential-assets", default-features = false }

# Verifier (no_std for on-chain use)
zkhe-verifier = { path = "path/to/zkhe/verifier", default-features = false }

# Primitives
confidential-assets-primitives = { path = "path/to/primitives/confidential-assets", default-features = false }

[features]
std = [
    "pallet-zkhe/std",
    "pallet-confidential-assets/std",
    "zkhe-verifier/std",
    "confidential-assets-primitives/std",
]
```

For client applications, add:

```toml
[dependencies]
zkhe-prover = { path = "path/to/zkhe/prover" }
```

## Step 2: Configure Pallets

In your runtime's `lib.rs`:

```rust
// Define asset and balance types
pub type AssetId = u128;
pub type Balance = u128;

// Configure pallet-zkhe (backend)
impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = ();
}

// Configure pallet-confidential-assets (interface)
impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;           // Use pallet-zkhe as backend
    type Ramp = PublicRamp;        // Your ramp implementation
    type AssetMetadata = ();       // Optional metadata provider
    type Acl = ();                 // Optional ACL (default: allow all)
    type Operators = ();           // Optional operators (default: none)
    type WeightInfo = ();
}

// Add to construct_runtime!
construct_runtime!(
    pub enum Runtime {
        // ... other pallets ...

        Zkhe: pallet_zkhe = 40,
        ConfidentialAssets: pallet_confidential_assets = 41,
    }
);
```

## Step 3: Implement the Ramp

The ramp bridges public and confidential assets:

```rust
use confidential_assets_primitives::Ramp;

pub struct PublicRamp;

impl Ramp<AccountId, AssetId, Balance> for PublicRamp {
    type Error = DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        // Transfer public assets between accounts
        Assets::transfer(asset, from, to, amount, Preservation::Expendable)?;
        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Mint public assets (for withdrawals)
        Assets::mint_into(*asset, to, amount)?;
        Ok(())
    }

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Burn public assets (for deposits)
        Assets::burn_from(*asset, from, amount, Precision::BestEffort, Fortitude::Polite)?;
        Ok(())
    }
}
```

## Step 4: Build and Test

```bash
# Build runtime
cargo build --release

# Run tests
cargo test -p pallet-zkhe -p pallet-confidential-assets

# Run with local node
./target/release/parachain-template-node --dev
```

## Step 5: Client Integration

Generate proofs client-side:

```rust
use zkhe_prover::{prove_sender_transfer, SenderInput};
use curve25519_dalek::scalar::Scalar;

// User's secret key (keep private!)
let sk = Scalar::from(12345u64);
let pk = sk * curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;

// Generate transfer proof
let input = SenderInput {
    asset_id: asset_id.encode(),
    network_id: [0u8; 32],
    sender_pk: pk,
    receiver_pk: recipient_pk,
    from_old_c: current_balance_commitment,
    from_old_opening: (balance, randomness),
    to_old_c: recipient_pending_commitment,
    delta_value: transfer_amount,
    rng_seed: secure_random_seed(),
    fee_c: None,
};

let output = prove_sender_transfer(&input)?;

// Submit to chain
api.tx()
    .confidential_assets()
    .confidential_transfer(
        asset_id,
        recipient,
        output.delta_ct_bytes,
        output.sender_bundle_bytes,
    )
    .sign_and_submit(&signer)
    .await?;
```

## Next Steps

- [Architecture Overview](./architecture.md) - Understand how it works
- [Asset Hub Integration](./asset-hub.md) - Deploy to production
- [Configuration Guide](./configuration.md) - Customize behavior
