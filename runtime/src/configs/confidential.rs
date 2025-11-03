//! Confidential Framework Implementation for Runtime.
use crate::{Balance, Runtime, RuntimeEvent, Zkhe};
use frame_support::{parameter_types, PalletId};
use polkadot_sdk::{frame_support, xcm_builder};
use xcm_builder::EnsureXcmOrigin;

// TODO: replace with pallet_assets and pallet_balances
pub type AssetId = u128;
parameter_types! {
    pub const EscrowPalletId: PalletId = PalletId(*b"CaEscrow");
    pub const BridgePalletId: PalletId = PalletId(*b"CaBridge");
}

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = ();
}
impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}

// impl pallet_confidential_bridge::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type AssetId = AssetId;
//     type Balance = Balance;
//     type Backend = Zkhe;
//     type Escrow = ConfidentialEscrow;
//     type Messenger = ();//HrmpMessenger;
//     type BurnPalletId = BridgePalletId;
//     type DefaultTimeout = ConstU32<{50}>;
//     // Origin allowed to confirm/cancel on behalf of destination responses.
//     type ConfirmOrigin = EnsureXcmOrigin<RuntimeOrigin, super::LocalOriginToLocation>;
//     type WeightInfo = ();
// }

// TODO: implement Ramp for Runtime using pallet-assets and pallet-balances

// impl pallet_confidential_assets::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type AssetId = AssetId;
//     type Balance = Balance;
//     type Backend = Zkhe;

//     // TODO
//     type Ramp: Ramp<Self::AccountId, Self::AssetId, Self::Balance>;
//     type AssetMetadata: AssetMetadataProvider<Self::AssetId>;

//     /// PalletId to derive the custodial account used for holding escrowed
//     /// balances iff `Self::Ramp` is implemented accordingly
//     #[pallet::constant]
//     type PalletId: Get<PalletId>;

//     /// Optional ACL (default = ()).
//     type Acl = ();
//     /// Operator layer. Defaults to always returning false when assigned ().
//     type Operators = ();

//     type WeightInfo = ();
// }

// // TODO: impl HrmpMessenger for Runtime

//
// // optional: pallet-acl, pallet-operators
