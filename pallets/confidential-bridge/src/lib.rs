//! **pallet-confidential-bridge**
//!
//! Goal: Bridge adapter that coordinates confidential, multi-asset
//! transfers *between* parachains using HRMP. On the source chain we
//! **escrow** a confidential ciphertext, send an HRMP packet to the
//! destination, and later **finalize** by either:
//! - success → move the escrowed ciphertext to this pallet’s burn account
//!   and **burn** it (supply conservation), or
//! - timeout/cancel → **refund** the ciphertext back to the sender.
//!
//! Notes:
//! - This pallet deliberately avoids hard dependencies on XCM types to keep
//!   compilation simple and runtimes flexible. It relies on a tiny `HrmpMessenger`
//!   trait that the runtime can implement using pallet-xcm (HRMP) or a thin
//!   adapter. The message payload is SCALE-encoded and opaque to this pallet
//!   once sent.
//! - We use `ConfidentialEscrow` and `ConfidentialBackend`:
//!   * escrow_lock / escrow_release / escrow_refund for custody flow,
//!   * burn_encrypted for post-success supply adjustment.
//! - The destination chain is expected to credit/mint the ciphertext (its own
//!   backend/pallet) and then send an HRMP response that eventually calls
//!   `confirm_success`. For simplicity, we also expose a `cancel_and_refund`
//!   path callable by the original sender after a deadline.
//!
//! This is intentionally minimal and should compile with standard Substrate
//! pallets in scope. Integrators can extend weights, origins, and message
//! formats without changing the basic flow.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Get, PalletId};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

use confidential_assets_primitives::{
    BridgePacket, ConfidentialBackend, ConfidentialEscrow, EncryptedAmount, HrmpMessenger,
    InputProof, PendingTransfer, TransferId,
};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Emit events.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Asset and balance types for the confidential backend.
        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
        type Balance: Parameter + Member + Copy + Default + MaxEncodedLen + TypeInfo;

        /// Confidential state/backend (read/verify/burn/mint/transfer).
        type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

        /// Confidential escrow adapter (lock, release, refund).
        type Escrow: ConfidentialEscrow<Self::AccountId, Self::AssetId>;

        /// HRMP messenger adapter (runtime supplies an implementation).
        type Messenger: HrmpMessenger;

        /// PalletId used to derive the *burn account* for finalization.
        /// We first escrow-release to this account (with a transfer proof),
        /// then burn from it (with a burn proof).
        #[pallet::constant]
        type BurnPalletId: Get<PalletId>;

        /// Default timeout in blocks for pending transfers.
        #[pallet::constant]
        type DefaultTimeout: Get<BlockNumberFor<Self>>;

        /// Origin allowed to confirm/cancel on behalf of destination responses.
        /// In production wire this to an XCM origin filter (e.g., EnsureXcm<…>).
        type ConfirmOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight info (minimal defaults provided below).
        type WeightInfo: WeightData;
    }

    /// Minimal weights (feel free to override in runtime).
    pub trait WeightData {
        fn send() -> Weight;
        fn confirm_success() -> Weight;
        fn cancel_and_refund() -> Weight;
    }
    impl WeightData for () {
        fn send() -> Weight {
            Weight::from_parts(50_000, 0)
        }
        fn confirm_success() -> Weight {
            Weight::from_parts(60_000, 0)
        }
        fn cancel_and_refund() -> Weight {
            Weight::from_parts(60_000, 0)
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn next_transfer_id)]
    pub type NextTransferId<T: Config> = StorageValue<_, TransferId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pending)]
    pub type Pending<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TransferId,
        PendingTransfer<T::AccountId, T::AssetId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    // --------------------------- Events / Errors --------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Outbound transfer was initiated and escrowed locally.
        TransferInitiated {
            id: TransferId,
            from: T::AccountId,
            dest_para: u32,
            asset: T::AssetId,
        },
        /// Destination reported success; local escrow burned (supply reduced).
        TransferConfirmed { id: TransferId, asset: T::AssetId },
        /// Sender reclaimed escrow after timeout or explicit cancel.
        TransferRefunded { id: TransferId, asset: T::AssetId },
        /// HRMP packet was attempted (success/failure), for observability.
        HrmpSent {
            id: TransferId,
            dest_para: u32,
            ok: bool,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
        AlreadyCompleted,
        NotExpired,
        MessengerFailed,
        /// Generic backend/escrow error path.
        BackendError,
        /// Only the original sender may self-cancel after deadline.
        NotSender,
    }

    // --------------------------- Helpers ----------------------------------------------

    impl<T: Config> Pallet<T> {
        #[inline]
        pub fn burn_account() -> T::AccountId {
            T::BurnPalletId::get().into_account_truncating()
        }

        #[inline]
        fn new_transfer_id() -> TransferId {
            let id = NextTransferId::<T>::get();
            NextTransferId::<T>::put(id.wrapping_add(1));
            id
        }
    }

    // --------------------------- Calls -------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initiate an outbound confidential bridge transfer.
        ///
        /// Flow (source chain):
        /// 1) Escrow: move encrypted amount from `who` into the *escrow* (via `Escrow::escrow_lock`).
        /// 2) HRMP: send a packet to `dest_para` containing the data destination needs
        ///    to accept/mint/credit the ciphertext (`accept_envelope` is opaque).
        ///
        /// Later:
        /// - Destination responds (via HRMP → runtime origin) calling `confirm_success`
        ///   with proofs to move escrow → burn account and then burn.
        /// - Or the sender cancels after the deadline with `cancel_and_refund`.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::send())]
        pub fn send_confidential(
            origin: T::RuntimeOrigin,
            dest_para: u32,
            dest_account: T::AccountId,
            asset: T::AssetId,
            encrypted_amount: EncryptedAmount,
            // Proof required by the escrow pallet to lock `encrypted_amount` from the sender.
            lock_proof: InputProof,
            // Opaque envelope/proof bytes for the **destination** chain to accept/credit.
            accept_envelope: InputProof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1) Escrow the ciphertext from `who`.
            T::Escrow::escrow_lock(asset, &who, encrypted_amount, lock_proof)
                .map_err(|_| Error::<T>::BackendError)?;

            // 2) Record pending transfer with deadline.
            let id = Self::new_transfer_id();
            let now = <frame_system::Pallet<T>>::block_number();
            let deadline = now + T::DefaultTimeout::get();

            Pending::<T>::insert(
                id,
                PendingTransfer::<T::AccountId, T::AssetId, BlockNumberFor<T>> {
                    from: who.clone(),
                    dest_para,
                    dest_account: dest_account.clone(),
                    asset,
                    encrypted_amount,
                    deadline,
                    completed: false,
                },
            );

            // 3) Send HRMP packet (opaque to this pallet).
            let packet = BridgePacket::<T::AccountId, T::AssetId> {
                transfer_id: id,
                dest_account,
                asset,
                encrypted_amount,
                accept_envelope,
            };
            let payload = packet.encode();
            let ok = T::Messenger::send(dest_para, payload).is_ok();
            if !ok {
                // Keep the pending record; sender may cancel/refund later.
                Self::deposit_event(Event::HrmpSent {
                    id,
                    dest_para,
                    ok: false,
                });
                // We do NOT roll back escrow to keep logic simple; the sender can cancel on timeout.
                // (Integrators can choose to eagerly refund here if desired.)
            } else {
                Self::deposit_event(Event::HrmpSent {
                    id,
                    dest_para,
                    ok: true,
                });
            }

            Self::deposit_event(Event::TransferInitiated {
                id,
                from: who,
                dest_para,
                asset,
            });
            Ok(())
        }

        /// Finalize a successful outbound transfer.
        ///
        /// Expected to be called from an XCM/HRMP verified origin on the source chain
        /// after the destination has credited/minted the ciphertext.
        ///
        /// Steps (source chain):
        /// 1) Move escrowed ciphertext to this pallet’s **burn account** using the provided
        ///    `release_proof` (escrow_release → underlying backend transfer).
        /// 2) Burn from the burn account using `burn_proof`.
        ///
        /// If both succeed, the pending record is cleared.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::confirm_success())]
        pub fn confirm_success(
            origin: T::RuntimeOrigin,
            id: TransferId,
            // Proof to release escrow → burn account (a standard confidential transfer proof).
            release_proof: InputProof,
            // Proof to burn from burn account (backend burn proof).
            burn_proof: InputProof,
        ) -> DispatchResult {
            // Ensure an origin authorized by the runtime (ideally an EnsureXcm origin).
            T::ConfirmOrigin::ensure_origin(origin)?;

            let mut rec = Pending::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(!rec.completed, Error::<T>::AlreadyCompleted);

            let burn_acc = <Pallet<T>>::burn_account();

            // 1) Release escrow to burn account.
            T::Escrow::escrow_release(rec.asset, &burn_acc, rec.encrypted_amount, release_proof)
                .map_err(|_| Error::<T>::BackendError)?;

            // 2) Burn from burn account (supply conservation).
            let _disclosed: T::Balance =
                T::Backend::burn_encrypted(rec.asset, &burn_acc, rec.encrypted_amount, burn_proof)
                    .map_err(|_| Error::<T>::BackendError)?;

            rec.completed = true;
            Pending::<T>::insert(id, &rec);
            Pending::<T>::remove(id);

            Self::deposit_event(Event::TransferConfirmed {
                id,
                asset: rec.asset,
            });
            Ok(())
        }

        /// Cancel and refund an outbound transfer after the deadline, or by a privileged
        /// origin (runtime choice).
        ///
        /// Steps:
        /// - If called by the original sender and the deadline has passed, refund escrow → sender.
        /// - If called by `ConfirmOrigin` at any time, refund escrow → sender.
        ///
        /// Requires a transfer proof (`refund_proof`) to move ciphertext from escrow
        /// back to the original `from`.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::cancel_and_refund())]
        pub fn cancel_and_refund(
            origin: T::RuntimeOrigin,
            id: TransferId,
            refund_proof: InputProof,
        ) -> DispatchResult {
            let caller = origin.clone();

            let mut rec = Pending::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(!rec.completed, Error::<T>::AlreadyCompleted);

            // Two options for authority:
            // 1) Original sender *after* deadline.
            // 2) Privileged confirm origin (e.g., an XCM admin) at any time.
            let sender_ok = if let Ok(who) = ensure_signed(caller.clone()) {
                ensure!(who == rec.from, Error::<T>::NotSender);
                let now = <frame_system::Pallet<T>>::block_number();
                ensure!(now >= rec.deadline, Error::<T>::NotExpired);
                true
            } else {
                // If not signed, require the confirm origin.
                T::ConfirmOrigin::ensure_origin(caller).is_ok()
            };
            ensure!(sender_ok, Error::<T>::NotExpired);

            // Refund escrow → original sender.
            T::Escrow::escrow_refund(rec.asset, &rec.from, rec.encrypted_amount, refund_proof)
                .map_err(|_| Error::<T>::BackendError)?;

            rec.completed = true;
            Pending::<T>::insert(id, &rec);
            Pending::<T>::remove(id);

            Self::deposit_event(Event::TransferRefunded {
                id,
                asset: rec.asset,
            });
            Ok(())
        }
    }
}
