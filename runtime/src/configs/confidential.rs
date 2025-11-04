//! Confidential Pallets Configuration
//!
//! Optional: pallet-acl, pallet-operators
use crate::{
    AccountId, Balance, ConfidentialEscrow, ParachainInfo, PolkadotXcm, Runtime, RuntimeCall,
    RuntimeEvent, RuntimeOrigin, Zkhe,
};
use alloc::{boxed::Box, vec, vec::Vec};
use confidential_assets_primitives::{HrmpMessenger, Ramp};
use frame_support::{parameter_types, traits::ConstU32, PalletId};
use parity_scale_codec::Encode;
use polkadot_sdk::{
    frame_support, sp_runtime, staging_xcm as xcm, staging_xcm_builder as xcm_builder,
};
use sp_runtime::{traits::AccountIdConversion, BoundedVec};
use xcm::latest::prelude::*;
use xcm::{VersionedLocation, VersionedXcm};
use xcm_builder::EnsureXcmOrigin;

pub type AssetId = u128;

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = ();
}

// ----------------- Confidential Assets Configuration -----------------

parameter_types! {
    pub const AssetsPalletId: PalletId = PalletId(*b"CaAssets");
}

pub struct PublicRamp;

impl Ramp<AccountId, AssetId, Balance> for PublicRamp {
    type Error = ();

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        Ok(())
    }
    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        Ok(())
    }
}
// implement Ramp for Runtime using pallet-assets and pallet-balances

impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = PublicRamp;
    type PalletId = AssetsPalletId;
    // should be configured using pallet-assets/balances for production not necessary for demo purposes
    type AssetMetadata = ();
    // Optional ACL (default = ()).
    type Acl = ();
    // Operator layer. Defaults to always returning false when assigned ().
    type Operators = ();
    type WeightInfo = ();
}

// ----------------- Confidential Bridge Configuration -----------------

parameter_types! {
    pub const EscrowPalletId: PalletId = PalletId(*b"CaEscrow");
    pub const BridgePalletId: PalletId = PalletId(*b"CaBridge");
    pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
}
fn bridge_account() -> AccountId {
    BridgePalletId::get().into_account_truncating()
}
/// HRMP messenger implementation used by confidential-bridge pallet. Assumes open channel exists.
pub struct XcmHrmpMessenger;
impl HrmpMessenger for XcmHrmpMessenger {
    fn send(dest_para: u32, payload: Vec<u8>) -> Result<(), ()> {
        let dest = Location::new(1, [Parachain(dest_para.into())]);
        let call = RuntimeCall::ConfidentialBridge(
            pallet_confidential_bridge::Call::<Runtime>::receive_confidential {
                payload: BoundedVec::try_from(payload).map_err(|_| ())?,
            },
        );
        let msg = Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            fallback_max_weight: None,
            call: call.encode().into(),
        }]);
        let origin = RuntimeOrigin::signed(bridge_account());
        PolkadotXcm::send(
            origin,
            Box::new(VersionedLocation::from(dest)),
            Box::new(VersionedXcm::from(msg)),
        )
        .map(|_| ())
        .map_err(|_| ())
    }
}

impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}
impl pallet_confidential_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Escrow = ConfidentialEscrow;
    type Messenger = XcmHrmpMessenger;
    type BurnPalletId = BridgePalletId;
    type DefaultTimeout = ConstU32<10>;
    type SelfParaId = SelfParaId;
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, super::LocalOriginToLocation>;
    type WeightInfo = ();
}
