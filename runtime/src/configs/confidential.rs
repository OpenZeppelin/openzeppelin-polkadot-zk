//! Confidential Pallets Configuration
//!
//! Optional: pallet-acl, pallet-operators
use crate::{AccountId, AssetId, Balance, Runtime, RuntimeEvent, Zkhe};
use confidential_assets_primitives::Ramp;
use frame_support::traits::{
    Currency, ExistenceRequirement, Get,
    tokens::fungibles::Mutate as MultiTransfer,
    tokens::{Fortitude, Precision, Preservation, WithdrawReasons},
};
use polkadot_sdk::{frame_support, pallet_assets, pallet_balances, sp_runtime};
use sp_runtime::DispatchError;

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier;
    type WeightInfo = pallet_zkhe::weights::WeightInfo<Runtime>;
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
    type WeightInfo = pallet_confidential_assets::weights::WeightInfo<Runtime>;
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
            // Native "mint": deposit_creating increases issuance, returns a PositiveImbalance which
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
            // Native "burn": withdraw with reasons; dropping the NegativeImbalance reduces issuance.
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
