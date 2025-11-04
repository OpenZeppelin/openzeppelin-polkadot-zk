//! Confidential Pallets Configuration
//!
//! Optional: pallet-acl, pallet-operators
use crate::{
    AccountId, AssetId, Balance, ConfidentialEscrow, ParachainInfo, PolkadotXcm, Runtime,
    RuntimeCall, RuntimeEvent, RuntimeOrigin, Zkhe,
};
use alloc::{boxed::Box, vec, vec::Vec};
use confidential_assets_primitives::{HrmpMessenger, Ramp};
use frame_support::traits::{
    tokens::fungibles::{Mutate as MultiMutate, Transfer as MultiTransfer},
    tokens::WithdrawReasons,
    Currency, ExistenceRequirement,
};
use frame_support::{
    parameter_types,
    traits::{ConstU32, Get},
    PalletId,
};
use parity_scale_codec::Encode;
use polkadot_sdk::{
    frame_support, pallet_assets, pallet_balances, sp_runtime, staging_xcm as xcm,
    staging_xcm_builder as xcm_builder,
};
use sp_runtime::{traits::AccountIdConversion, BoundedVec};
use xcm::latest::prelude::*;
use xcm::{VersionedLocation, VersionedXcm};
use xcm_builder::EnsureXcmOrigin;

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = ();
}
impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = PublicRamp;
    type PalletId = AssetsPalletId;
    // prod configure using pallet-assets/balances but not needed for demo
    type AssetMetadata = ();
    // Optional ACL (default = ()).
    type Acl = ();
    // Operator layer. Defaults to always returning false when assigned ().
    type Operators = ();
    type WeightInfo = ();
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

// ----------------- Confidential Assets Helpers -----------------

parameter_types! {
    pub const AssetsPalletId: PalletId = PalletId(*b"CaAssets");
}

pub struct NativeAssetId;
impl Get<AssetId> for NativeAssetId {
    fn get() -> AssetId {
        0u32.into()
    } // assumes Native Asset Id is 0
}

#[inline]
fn is_native(asset: &AssetId) -> bool {
    *asset == NativeAssetId::get()
}

type Balances = pallet_balances::Pallet<Runtime>;
type Assets = pallet_assets::Pallet<Runtime>;

pub struct PublicRamp;
impl Ramp<AccountId, AssetId, Balance> for PublicRamp {
    type Error = DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        if is_native(&asset) {
            // Native: via Currency
            <Balances as Currency<AccountId>>::transfer(
                from,
                to,
                amount,
                ExistenceRequirement::AllowDeath,
            )?;
        } else {
            // Non-native: via fungibles::Transfer on pallet_assets
            <Assets as MultiTransfer<AccountId>>::transfer(
                asset, from, to, amount, /* keep_alive: */ true,
            )?;
        }
        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        if is_native(asset) {
            // Native “mint”: deposit_creating increases issuance, returns a PositiveImbalance which
            // is burned when dropped if your Currency implements Balanced. Just ignore it here.
            let _imbalance = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
            Ok(())
        } else {
            // Non-native mint
            <Assets as MultiMutate<AccountId>>::mint_into(*asset, to, amount)?;
            Ok(())
        }
    }

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        if is_native(asset) {
            // Native “burn”: withdraw with reasons; dropping the NegativeImbalance reduces issuance.
            let _imbalance = <Balances as Currency<AccountId>>::withdraw(
                from,
                amount,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok(())
        } else {
            // Non-native burn
            <Assets as MultiMutate<AccountId>>::burn_from(*asset, from, amount)?;
            Ok(())
        }
    }
}

// ----------------- Confidential Bridge Helpers -----------------

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
