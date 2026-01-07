# Pallet Configuration

Complete reference for configuring confidential assets pallets.

## pallet-confidential-assets

The main interface pallet following ERC-7984.

### Config Trait

```rust
pub trait Config: frame_system::Config {
    /// Runtime event type
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Asset identifier type (e.g., u32, u128)
    type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;

    /// Balance type (typically u128)
    type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default;

    /// Cryptographic backend for encrypted operations
    /// Implementations: pallet_zkhe, or custom FHE/TEE backend
    type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

    /// Bridge between public and confidential assets
    type Ramp: Ramp<Self::AccountId, Self::AssetId, Self::Balance>;

    /// Asset metadata provider (name, symbol, decimals)
    /// Use () for no metadata
    type AssetMetadata: AssetMetadataProvider<Self::AssetId>;

    /// Access control for transfers
    /// Use () to allow all transfers
    type Acl: AclProvider<Self::AccountId, Self::AssetId, Self::Balance>;

    /// Operator permissions (for delegated transfers)
    /// Use () for no operator support
    type Operators: OperatorRegistry<Self::AccountId, Self::AssetId, BlockNumberFor<Self>>;

    /// Weight information
    type WeightInfo: WeightData;
}
```

### Configuration Examples

**Minimal configuration:**

```rust
impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = u128;
    type Backend = Zkhe;
    type Ramp = SimpleRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}
```

**Full-featured configuration:**

```rust
impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = u128;
    type Backend = Zkhe;
    type Ramp = AssetHubRamp;
    type AssetMetadata = PalletAssetsMetadata;
    type Acl = AclPallet;
    type Operators = OperatorsPallet;
    type WeightInfo = weights::SubstrateWeight<Runtime>;
}
```

## pallet-zkhe

The ZK-ElGamal cryptographic backend.

### Config Trait

```rust
pub trait Config: frame_system::Config {
    /// Runtime event type
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Asset identifier type
    type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;

    /// Balance type
    type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default + Into<u64>;

    /// ZK proof verifier implementation
    type Verifier: ZkVerifier;

    /// Weight information
    type WeightInfo: WeightInfo;
}
```

### Configuration Example

```rust
impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = u128;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = ();
}
```

## pallet-confidential-escrow

Escrow management for cross-chain operations.

### Config Trait

```rust
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
    type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default;
    type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

    /// Pallet ID for deriving escrow account
    type PalletId: Get<PalletId>;
}
```

### Configuration Example

```rust
parameter_types! {
    pub const EscrowPalletId: PalletId = PalletId(*b"CaEscrow");
}

impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = u128;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}
```

## pallet-confidential-bridge

Cross-chain confidential transfers via XCM.

### Config Trait

```rust
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
    type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default;
    type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;
    type Escrow: ConfidentialEscrow<Self::AccountId, Self::AssetId, Self::Balance>;

    /// XCM message sender
    type Messenger: HrmpMessenger;

    /// Maximum proof payload size
    type MaxBridgePayload: Get<u32>;

    /// Pallet ID for burn account
    type BurnPalletId: Get<PalletId>;

    /// Default timeout in blocks
    type DefaultTimeout: Get<u32>;

    /// This parachain's ID
    type SelfParaId: Get<u32>;

    /// XCM origin converter
    type XcmOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    type WeightInfo: WeightInfo;
}
```

### Configuration Example

```rust
parameter_types! {
    pub const MaxBridgePayload: u32 = 16 * 1024;  // 16 KiB
    pub const BridgePalletId: PalletId = PalletId(*b"CaBridge");
    pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
}

impl pallet_confidential_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = u128;
    type Backend = Zkhe;
    type Escrow = ConfidentialEscrow;
    type Messenger = XcmHrmpMessenger;
    type MaxBridgePayload = MaxBridgePayload;
    type BurnPalletId = BridgePalletId;
    type DefaultTimeout = ConstU32<100>;
    type SelfParaId = SelfParaId;
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type WeightInfo = ();
}
```

## Key Parameters

### Payload Sizes

```rust
// Bounded vectors for proof data
pub const MAX_PUBLIC_KEY_SIZE: u32 = 64;      // 64 bytes
pub const MAX_ENCRYPTED_AMOUNT: u32 = 64;     // 64 bytes
pub const MAX_PROOF_SIZE: u32 = 16 * 1024;    // 16 KiB
```

### Pallet IDs

```rust
// Standard pallet IDs (8 bytes each)
pub const ESCROW_PALLET_ID: [u8; 8] = *b"CaEscrow";
pub const BRIDGE_PALLET_ID: [u8; 8] = *b"CaBridge";
```

### Timeouts

```rust
// Cross-chain operation timeouts
pub const DEFAULT_BRIDGE_TIMEOUT: u32 = 100;  // ~10 minutes at 6s blocks
pub const MAX_BRIDGE_TIMEOUT: u32 = 14400;    // ~24 hours
```

## Weight Configuration

Weights should be generated via benchmarking for production deployments:

```bash
cargo build --release --features runtime-benchmarks

./target/release/parachain-template-node benchmark pallet \
    --chain dev \
    --pallet pallet_confidential_assets \
    --extrinsic '*' \
    --output weights.rs
```

## Storage Configuration

### Storage Deposits

Configure existential deposits for confidential accounts:

```rust
parameter_types! {
    pub const ConfidentialExistentialDeposit: Balance = MILLI_UNIT;
}
```

### Storage Limits

```rust
parameter_types! {
    // Maximum pending deposits per account per asset
    pub const MaxPendingDeposits: u32 = 100;

    // Maximum operators per account
    pub const MaxOperators: u32 = 10;
}
```

## Environment Variables

For the prover and development:

```bash
# Prover RNG seed for deterministic testing
export ZKHE_RNG_SEED="0x0102030405060708..."

# Network ID for transcript binding
export ZKHE_NETWORK_ID="0x00000000..."

# Enable debug output
export RUST_LOG=zkhe_prover=debug,zkhe_verifier=debug
```

## Next Steps

- [Runtime Integration](./runtime-integration.md) - Complete runtime setup
- [Custom Backends](./custom-backends.md) - Implement custom backends
- [ACL & Operators](./acl-operators.md) - Configure access control
