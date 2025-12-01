//! Confidential Pallets Configuration
//!
//! Optional: pallet-acl, pallet-operators
use crate::{
    AccountId, AssetId, Balance, ConfidentialEscrow, ParachainInfo, PolkadotXcm, Runtime,
    RuntimeCall, RuntimeEvent, RuntimeOrigin, Zkhe,
};
use alloc::{boxed::Box, vec, vec::Vec};
use confidential_assets_primitives::{HrmpMessenger, NetworkIdProvider, Ramp};
use frame_support::traits::{
    tokens::fungibles::Mutate as MultiTransfer,
    tokens::{Fortitude, Precision, Preservation, WithdrawReasons},
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
use sp_runtime::{
    DispatchError,
    {traits::AccountIdConversion, BoundedVec},
};
use xcm::latest::prelude::*;
use xcm::{VersionedLocation, VersionedXcm};
use xcm_builder::EnsureXcmOrigin;

/// Network ID provider for this runtime.
///
/// In production, this should return a unique identifier for the network (e.g., genesis hash
/// or a configured chain-specific ID) to provide domain separation for ZK proofs.
pub struct RuntimeNetworkId;
impl NetworkIdProvider for RuntimeNetworkId {
    fn network_id() -> [u8; 32] {
        // TODO: Replace with actual network identifier (e.g., from genesis config)
        // For now returns zeros to maintain compatibility with test vectors
        [0u8; 32]
    }
}

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier<RuntimeNetworkId>;
    type WeightInfo = ();
}
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
impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}
parameter_types! {
    pub const MaxBridgePayload: u32 = 16 * 1024; // 16 KiB is safe for two Bulletproofs, link proof, etc.
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
    type DefaultTimeout = ConstU32<10>;
    type SelfParaId = SelfParaId;
    type XcmOrigin = EnsureXcmOrigin<RuntimeOrigin, super::LocalOriginToLocation>;
    type WeightInfo = ();
}

// ----------------- Confidential Assets Helpers -----------------

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
                asset,
                from,
                to,
                amount,
                Preservation::Expendable,
            )?;
        }
        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        if is_native(asset) {
            // Native “mint”: deposit_creating increases issuance, returns a PositiveImbalance which
            // is burned when dropped if your Currency implements Balanced. Just ignore it here.
            let _imbalance = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
        } else {
            // Non-native mint
            <Assets as MultiTransfer<AccountId>>::mint_into(*asset, to, amount)?;
        }
        Ok(())
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
        } else {
            // Non-native burn
            <Assets as MultiTransfer<AccountId>>::burn_from(
                *asset,
                from,
                amount,
                Preservation::Expendable,
                Precision::BestEffort,
                Fortitude::Polite,
            )?;
        }
        Ok(())
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
        // Use the SAME bound as the pallet call expects:
        let payload_bv: BoundedVec<u8, MaxBridgePayload> =
            BoundedVec::try_from(payload).map_err(|_| ())?;

        let dest = (Parent, Parachain(dest_para));
        let call = RuntimeCall::ConfidentialBridge(
            pallet_confidential_bridge::Call::<Runtime>::receive_confidential {
                payload: payload_bv,
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
