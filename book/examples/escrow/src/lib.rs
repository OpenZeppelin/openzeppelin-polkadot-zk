//! pallet-escrow-trust — escrow adapter for multi-asset runtimes.
//! Assumes `confidential_assets_primitives::EscrowTrust` trait exists and is imported.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{
    PalletId,
    pallet_prelude::*,
    traits::{
        Get,
        fungibles::{Inspect, Mutate},
        tokens::Preservation,
    },
};
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

use confidential_assets_primitives::EscrowTrust;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Identifier of assets handled by `Assets`.
        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen;

        /// Balance value type for all assets.
        type Balance: Parameter
            + Member
            + Copy
            + Default
            + MaxEncodedLen
            + sp_runtime::traits::AtLeast32BitUnsigned
            + sp_runtime::traits::Saturating
            + sp_runtime::traits::CheckedAdd
            + sp_runtime::traits::CheckedSub
            + PartialOrd;

        /// Multi-asset ledger used to move funds into/out of escrow.
        /// This should typically be `pallet_assets::Pallet<T>` or a similar fungibles implementation.
        type Assets: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

        /// PalletId used to derive the escrow account (like Treasury).
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    /// Total amount held in escrow per asset (kept deliberately simple).
    #[pallet::storage]
    #[pallet::getter(fn escrow_total)]
    pub type EscrowTotal<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, T::Balance, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Funds moved into escrow.
        EscrowLocked {
            asset: T::AssetId,
            from: T::AccountId,
            amount: T::Balance,
        },
        /// Funds released from escrow to beneficiary (successful redeem).
        EscrowReleased {
            asset: T::AssetId,
            to: T::AccountId,
            amount: T::Balance,
        },
        /// Funds refunded from escrow (e.g., after timeout).
        EscrowRefunded {
            asset: T::AssetId,
            to: T::AccountId,
            amount: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not enough total escrowed balance for this asset to cover the release/refund.
        InsufficientEscrow,
        /// Arithmetic overflow on accounting (should be rare; indicates config/balance size mismatch).
        Overflow,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    impl<T: Config> Pallet<T> {
        #[inline]
        pub fn escrow_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        #[inline]
        fn inc_total(asset: T::AssetId, by: T::Balance) -> Result<(), DispatchError> {
            EscrowTotal::<T>::try_mutate(asset, |total| {
                *total = total.checked_add(&by).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })
        }

        #[inline]
        fn dec_total(asset: T::AssetId, by: T::Balance) -> Result<(), DispatchError> {
            EscrowTotal::<T>::try_mutate(asset, |total| {
                // simple underflow check
                if *total < by {
                    return Err(Error::<T>::InsufficientEscrow.into());
                }
                *total = total.checked_sub(&by).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })
        }
    }

    impl<T: Config> EscrowTrust<T::AccountId, T::AssetId, T::Balance> for Pallet<T> {
        /// Move value from `who` into the pallet's escrow account.
        fn escrow_lock(
            asset: T::AssetId,
            who: &T::AccountId,
            amount: T::Balance,
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();

            // Move tokens from `who` -> escrow account.
            // Use Preservation::Preserve to avoid unintended provider/consumer changes.
            <T as Config>::Assets::transfer(asset, who, &escrow, amount, Preservation::Preserve)?;

            // Accounting
            Self::inc_total(asset, amount)?;

            // Emit event
            Self::deposit_event(Event::EscrowLocked {
                asset,
                from: who.clone(),
                amount,
            });

            Ok(())
        }

        /// Release escrowed value to `to` (on successful redeem).
        fn escrow_release(
            asset: T::AssetId,
            to: &T::AccountId,
            amount: T::Balance,
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();

            // Ensure we have enough total escrow recorded.
            Self::dec_total(asset, amount)?;

            // Move tokens from escrow -> beneficiary.
            <T as Config>::Assets::transfer(asset, &escrow, to, amount, Preservation::Preserve)?;

            Self::deposit_event(Event::EscrowReleased {
                asset,
                to: to.clone(),
                amount,
            });

            Ok(())
        }

        /// Refund escrowed value to `to` (after timeout).
        fn escrow_refund(
            asset: T::AssetId,
            to: &T::AccountId,
            amount: T::Balance,
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();

            // Ensure we have enough total escrow recorded.
            Self::dec_total(asset, amount)?;

            // Move tokens from escrow -> refund recipient.
            <T as Config>::Assets::transfer(asset, &escrow, to, amount, Preservation::Preserve)?;

            Self::deposit_event(Event::EscrowRefunded {
                asset,
                to: to.clone(),
                amount,
            });

            Ok(())
        }
    }
}
