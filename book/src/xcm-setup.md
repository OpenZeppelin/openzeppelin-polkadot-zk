# XCM Setup

Configure cross-chain confidential transfers using XCM.

## Overview

Cross-chain confidential transfers work via a two-phase commit:

1. **Source chain**: Escrow confidential assets, send mint proof via XCM
2. **Destination chain**: Mint confidential assets, send confirmation
3. **Source chain**: Release escrow on confirmation (or refund on timeout)

```
Source Parachain              Destination Parachain
┌─────────────────┐          ┌─────────────────┐
│ 1. escrow(Δ)    │─── XCM ──▶│ 2. mint(Δ)      │
│                 │          │                 │
│ 4. release()    │◀── XCM ──│ 3. confirm()    │
│    or timeout() │          │                 │
└─────────────────┘          └─────────────────┘
```

## Required Pallets

Both chains need:

```rust
// Source chain
Zkhe: pallet_zkhe,
ConfidentialAssets: pallet_confidential_assets,
ConfidentialEscrow: pallet_confidential_escrow,
ConfidentialBridge: pallet_confidential_bridge,

// Destination chain (same pallets)
Zkhe: pallet_zkhe,
ConfidentialAssets: pallet_confidential_assets,
ConfidentialEscrow: pallet_confidential_escrow,
ConfidentialBridge: pallet_confidential_bridge,
```

## Escrow Pallet Configuration

```rust
parameter_types! {
    pub const EscrowPalletId: PalletId = PalletId(*b"CaEscrow");
}

impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}
```

## Bridge Pallet Configuration

```rust
parameter_types! {
    // Maximum size for proof payloads in XCM messages
    pub const MaxBridgePayload: u32 = 16 * 1024;  // 16 KiB

    // Pallet ID for the burn account
    pub const BridgePalletId: PalletId = PalletId(*b"CaBridge");

    // This parachain's ID
    pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
}

impl pallet_confidential_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Escrow = ConfidentialEscrow;
    type Messenger = XcmHrmpMessenger;
    type MaxBridgePayload = MaxBridgePayload;
    type BurnPalletId = BridgePalletId;
    type DefaultTimeout = ConstU32<100>;  // ~10 minutes at 6s blocks
    type SelfParaId = SelfParaId;
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type WeightInfo = ();
}
```

## HRMP Messenger Implementation

The messenger sends XCM messages via HRMP:

```rust
use parity_scale_codec::Encode;
use xcm::latest::prelude::*;

pub struct XcmHrmpMessenger;

impl HrmpMessenger for XcmHrmpMessenger {
    fn send(dest_para: u32, payload: Vec<u8>) -> Result<(), ()> {
        // Bound the payload
        let payload_bv: BoundedVec<u8, MaxBridgePayload> =
            BoundedVec::try_from(payload).map_err(|_| ())?;

        // Construct destination
        let dest = Location::new(1, [Parachain(dest_para)]);

        // Create the call to be executed on destination
        let call = RuntimeCall::ConfidentialBridge(
            pallet_confidential_bridge::Call::<Runtime>::receive_confidential {
                payload: payload_bv,
            },
        );

        // Wrap in XCM Transact
        let msg = Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            fallback_max_weight: Some(Weight::from_parts(1_000_000_000, 0)),
            call: call.encode().into(),
        }]);

        // Get bridge account as origin
        let origin = RuntimeOrigin::signed(
            BridgePalletId::get().into_account_truncating()
        );

        // Send via pallet-xcm
        PolkadotXcm::send(
            origin,
            Box::new(VersionedLocation::from(dest)),
            Box::new(VersionedXcm::from(msg)),
        )
        .map(|_| ())
        .map_err(|_| ())
    }
}
```

## XCM Config Updates

Update your XCM configuration to handle confidential bridge calls:

```rust
// In xcm_config.rs

pub struct XcmConfig;

impl xcm_executor::Config for XcmConfig {
    // ... other config ...

    // Allow transact from sibling parachains
    type SafeCallFilter = SafeCallFilter;

    // Handle incoming calls
    type CallDispatcher = RuntimeCall;
}

// Define safe calls (include confidential bridge)
pub struct SafeCallFilter;
impl Contains<RuntimeCall> for SafeCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(
            call,
            RuntimeCall::ConfidentialBridge(
                pallet_confidential_bridge::Call::receive_confidential { .. } |
                pallet_confidential_bridge::Call::confirm_success { .. } |
                pallet_confidential_bridge::Call::confirm_failure { .. }
            )
        )
    }
}
```

## Origin Conversion

Ensure XCM origins are properly converted:

```rust
// Convert XCM origins to local origins
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account of sibling parachain
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native XCM origin for pallet-xcm
    XcmPassthrough<RuntimeOrigin>,
);

// For EnsureXcmOrigin in bridge config
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;
```

## HRMP Channel Setup

Ensure HRMP channels are open between parachains:

```rust
// This is typically done via relay chain governance

// Open channel from ParaA (1000) to ParaB (2000)
hrmp.hrmp_init_open_channel {
    recipient: 2000,
    proposed_max_capacity: 1000,
    proposed_max_message_size: 102400,
}

// ParaB accepts
hrmp.hrmp_accept_open_channel {
    sender: 1000,
}

// Open reverse channel
hrmp.hrmp_init_open_channel {
    recipient: 1000,
    proposed_max_capacity: 1000,
    proposed_max_message_size: 102400,
}

// ParaA accepts
hrmp.hrmp_accept_open_channel {
    sender: 2000,
}
```

## Cross-Chain Transfer Flow

### 1. Initiate Transfer (Source Chain)

```rust
// Client generates proofs
let sender_proof = zkhe_prover::prove_sender_transfer(&sender_input)?;
let mint_proof = zkhe_prover::prove_mint(&mint_input)?;

// Submit to source chain
ConfidentialBridge::send_confidential(
    origin,
    dest_para_id,      // Destination parachain
    recipient,         // Recipient on destination
    asset_id,
    delta_ct,          // Encrypted transfer amount
    sender_proof,      // Sender bundle (escrow proof)
    mint_proof,        // Mint proof for destination
)?;
```

### 2. Receive on Destination

The destination chain automatically processes `receive_confidential`:

```rust
// Executed via XCM Transact
ConfidentialBridge::receive_confidential(
    xcm_origin,
    payload,  // Contains mint proof + transfer details
)?;
// Emits: InboundTransferExecuted { id, sender, recipient, ... }
```

### 3. Confirm and Release

After confirmation on destination, source releases escrow:

```rust
// Client generates release/burn proofs
let release_proof = zkhe_prover::prove_receiver_accept(&accept_input)?;
let burn_proof = zkhe_prover::prove_burn(&burn_input)?;

// Send confirmation back via XCM
ConfidentialBridge::confirm_success(
    xcm_origin,  // Must be XCM origin from destination
    transfer_id,
    release_proof,
    burn_proof,
)?;
```

## Error Handling

### Timeout Refunds

If destination doesn't confirm within timeout:

```rust
// Anyone can trigger refund after timeout
ConfidentialBridge::timeout_transfer(
    origin,
    transfer_id,
)?;
// Returns escrowed funds to sender
```

### Failure Confirmation

Destination can explicitly fail:

```rust
ConfidentialBridge::confirm_failure(
    xcm_origin,
    transfer_id,
    reason: FailureReason,
)?;
// Triggers refund on source
```

## Testing XCM Integration

Use the XCM simulator for testing:

```rust
// In xcm/src/tests.rs

use xcm_simulator::TestExt;

#[test]
fn cross_chain_confidential_transfer() {
    MockNet::reset();

    // Setup on ParaA
    ParaA::execute_with(|| {
        // Register PKs, deposit, etc.
    });

    // Setup on ParaB
    ParaB::execute_with(|| {
        // Register recipient PK
    });

    // Initiate transfer
    ParaA::execute_with(|| {
        assert_ok!(ConfidentialBridge::send_confidential(...));
    });

    // Verify receipt
    ParaB::execute_with(|| {
        // Check events for InboundTransferExecuted
    });
}
```

## Security Considerations

### Origin Verification

Always verify XCM origins:

```rust
impl pallet_confidential_bridge::Config for Runtime {
    // Only accept XCM origins for critical operations
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
}
```

### Payload Size Limits

Limit proof payload sizes:

```rust
parameter_types! {
    // Don't accept oversized payloads
    pub const MaxBridgePayload: u32 = 16 * 1024;
}
```

### Channel Validation

Validate source parachain:

```rust
// In receive_confidential
ensure!(
    allowed_source_parachains.contains(&source_para),
    Error::UnauthorizedSource
);
```

## Monitoring

Track cross-chain transfers:

```rust
// Events to monitor
Event::OutboundTransferInitiated { id, dest_para, recipient, .. }
Event::InboundTransferExecuted { id, source_para, sender, .. }
Event::TransferConfirmed { id, .. }
Event::TransferTimedOut { id, .. }
Event::TransferFailed { id, reason, .. }
```

## Next Steps

- [Custom Backends](./custom-backends.md) - Alternative cryptographic backends
- [Testing Guide](./testing.md) - Test your XCM integration
- [Client Integration](./client.md) - Build cross-chain UIs
