//! pallet-confidential-escrow — escrow adapter that moves encrypted balances
//! via the ConfidentialBackend. It shares the same EscrowTrust trait by
//! assigning generic `Balance = (EncryptedAmount, InputProof)`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::pallet_prelude::*;
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

use confidential_assets_primitives::{
    ConfidentialBackend, EncryptedAmount, EscrowTrust, InputProof,
};
use frame_support::PalletId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen;
        // Required by ConfidentialBackend otherwise unused
        type Balance: Parameter + Member + Copy + Default + MaxEncodedLen;

        type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

        /// PalletId to derive a stable escrow account.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        EscrowLocked {
            asset: T::AssetId,
            from: T::AccountId,
            encrypted_amount: EncryptedAmount,
        },
        EscrowReleased {
            asset: T::AssetId,
            to: T::AccountId,
            encrypted_amount: EncryptedAmount,
        },
        EscrowRefunded {
            asset: T::AssetId,
            to: T::AccountId,
            encrypted_amount: EncryptedAmount,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        BackendError,
    }

    impl<T: Config> Pallet<T> {
        #[inline]
        pub fn escrow_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }

    // ------------ EscrowTrust (confidential specialization) ------------
    //
    // Param is exactly what the backend needs to authorize the transfer:
    // (EncryptedAmount delta, InputProof proving its validity).
    //
    impl<T: Config> EscrowTrust<T::AccountId, T::AssetId, (EncryptedAmount, InputProof)> for Pallet<T> {
        fn escrow_lock(
            asset: T::AssetId,
            who: &T::AccountId,
            (delta, proof): (EncryptedAmount, InputProof),
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();
            let encrypted = T::Backend::transfer_encrypted(asset, who, &escrow, delta, proof)
                .map_err(|_| Error::<T>::BackendError)?;
            Self::deposit_event(Event::EscrowLocked {
                asset,
                from: who.clone(),
                encrypted_amount: encrypted,
            });
            Ok(())
        }

        fn escrow_release(
            asset: T::AssetId,
            to: &T::AccountId,
            (delta, proof): (EncryptedAmount, InputProof),
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();
            let encrypted = T::Backend::transfer_encrypted(asset, &escrow, to, delta, proof)
                .map_err(|_| Error::<T>::BackendError)?;
            Self::deposit_event(Event::EscrowReleased {
                asset,
                to: to.clone(),
                encrypted_amount: encrypted,
            });
            Ok(())
        }

        fn escrow_refund(
            asset: T::AssetId,
            to: &T::AccountId,
            (delta, proof): (EncryptedAmount, InputProof),
        ) -> Result<(), DispatchError> {
            let escrow = Self::escrow_account();
            let encrypted = T::Backend::transfer_encrypted(asset, &escrow, to, delta, proof)
                .map_err(|_| Error::<T>::BackendError)?;
            Self::deposit_event(Event::EscrowRefunded {
                asset,
                to: to.clone(),
                encrypted_amount: encrypted,
            });
            Ok(())
        }
    }
}
