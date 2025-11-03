//! Confidential Framework Implementation for Runtime.

// 1. add all to Cargo.toml including zkhe_verifier
// 2. implement all for Runtime
use crate::{
    RuntimeEvent, AssetId, Balance, Runtime,
};


impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
}

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

// impl pallet_confidential_escrow::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type AssetId = AssetId;
//     type Balance = Balance;
//     type Backend = Zkhe;

//     #[pallet::constant]
//     type PalletId: Get<PalletId>;
// }

// // TODO: impl HrmpMessenger for Runtime

// impl pallet_confidential_bridge::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type AssetId = AssetId;
//     type Balance = Balance;
//     type Backend = Zkhe;
//     type Escrow = ConfidentialEscrow;

//     /// HRMP messenger adapter (runtime supplies an implementation).
//     type Messenger: HrmpMessenger;

//     /// PalletId used to derive the *burn account* for finalization.
//     /// We first escrow-release to this account (with a transfer proof),
//     /// then burn from it (with a burn proof).
//     #[pallet::constant]
//     type BurnPalletId: Get<PalletId>;

//     /// Default timeout in blocks for pending transfers.
//     #[pallet::constant]
//     type DefaultTimeout: Get<BlockNumberFor<Self>>;

//     /// Origin allowed to confirm/cancel on behalf of destination responses.
//     /// In production wire this to an XCM origin filter (e.g., EnsureXcm<…>).
//     type ConfirmOrigin: EnsureOrigin<Self::RuntimeOrigin>;

//     /// Weight info (minimal defaults provided below).
//     type WeightInfo = ();
// }
//
// // optional: pallet-acl, pallet-operators
