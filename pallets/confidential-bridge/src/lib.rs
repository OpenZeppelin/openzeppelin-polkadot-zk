//! THIS IS INTENTIONALLY UNSAFE FOR DEMO PURPOSES DO NOT USE IN PRODUCTION
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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::{pallet_prelude::*, traits::Get, transactional, PalletId};
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

        // ---------------------------- Confidential Assets Types and Traits ----------------------------

        /// Asset and balance types for the confidential backend.
        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
        type Balance: Parameter + Member + Copy + Default + MaxEncodedLen + TypeInfo;

        /// Confidential state/backend (read/verify/burn/mint/transfer).
        type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

        /// Confidential escrow adapter (lock, release, refund).
        type Escrow: ConfidentialEscrow<Self::AccountId, Self::AssetId>;

        // ---------------------------- XCM Types and Traits ----------------------------

        /// Origin allowed to confirm/cancel on behalf of destination responses.
        /// In production wire this to an XCM origin filter (e.g., EnsureXcm<…>).
        type XcmOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// HRMP messenger adapter (runtime supplies an implementation).
        type Messenger: HrmpMessenger;

        /// Maximum size in bytes for a bridge HRMP payload.
        type MaxBridgePayload: Get<u32>;

        #[pallet::constant]
        type SelfParaId: Get<u32>; // in prod use compact encoded u32: polkadot_parachain_primitives::Id

        /// PalletId used to derive the *burn account* for finalization.
        /// We first escrow-release to this account (with a transfer proof),
        /// then burn from it (with a burn proof).
        #[pallet::constant]
        type BurnPalletId: Get<PalletId>;

        /// Default timeout in blocks for pending transfers.
        #[pallet::constant]
        type DefaultTimeout: Get<BlockNumberFor<Self>>;

        /// Weight info (minimal defaults provided below).
        type WeightInfo: WeightData;
    }

    /// Minimal weights (feel free to override in runtime).
    pub trait WeightData {
        fn send() -> Weight;
        fn confirm_success() -> Weight;
        fn cancel_and_refund() -> Weight;
        fn receive() -> Weight;
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
        fn receive() -> Weight {
            Weight::from_parts(100_000, 0)
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
        OutboundTransferInitiated {
            id: TransferId,
            from: T::AccountId,
            dest_para: u32,
            asset: T::AssetId,
        },
        /// Destination reported success; local escrow burned (supply reduced).
        OutboundTransferConfirmed { id: TransferId, asset: T::AssetId },
        /// Sender reclaimed escrow after timeout or explicit cancel.
        OutboundTransferRefunded { id: TransferId, asset: T::AssetId },
        /// Incoming Transfer Executed
        InboundTransferExecuted {
            id: TransferId,
            asset: T::AssetId,
            minted: EncryptedAmount,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
        NotExpired,
        NotSender,
        NoSelfBridge,
        AlreadyCompleted,
        MessengerFailed,
        BackendError,
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
        #[transactional]
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
            ensure!(T::SelfParaId::get() != dest_para, Error::<T>::NoSelfBridge);
            let id = Self::new_transfer_id();
            let packet = BridgePacket::<T::AccountId, T::AssetId> {
                transfer_id: id,
                dest_account: dest_account.clone(),
                asset,
                encrypted_amount,
                accept_envelope,
            };
            let payload = packet.encode();
            ensure!(
                T::Messenger::send(dest_para, payload).is_ok(),
                Error::<T>::MessengerFailed
            );
            T::Escrow::escrow_lock(asset, &who, encrypted_amount, lock_proof)
                .map_err(|_| Error::<T>::BackendError)?;
            // Insert Pending Transfer Into Storage
            let deadline = <frame_system::Pallet<T>>::block_number() + T::DefaultTimeout::get();
            Pending::<T>::insert(
                id,
                PendingTransfer::<T::AccountId, T::AssetId, BlockNumberFor<T>> {
                    from: who.clone(),
                    dest_para,
                    dest_account,
                    asset,
                    encrypted_amount,
                    deadline,
                    completed: false,
                },
            );
            Self::deposit_event(Event::OutboundTransferInitiated {
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
        #[transactional]
        pub fn confirm_success(
            origin: T::RuntimeOrigin,
            id: TransferId,
            // Proof to release escrow → burn account (a standard confidential transfer proof).
            release_proof: InputProof,
            // Proof to burn from burn account (backend burn proof).
            burn_proof: InputProof,
        ) -> DispatchResult {
            T::XcmOrigin::ensure_origin(origin)?;

            let rec = Pending::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(!rec.completed, Error::<T>::AlreadyCompleted);

            let burn_acc = <Pallet<T>>::burn_account();

            let res1 = T::Escrow::escrow_release(rec.asset, &burn_acc, rec.encrypted_amount, release_proof);
            if res1.is_err() {
                println!("escrow failed");
                return Err(Error::<T>::AlreadyCompleted.into());
            }

            let res2 =
                T::Backend::burn_encrypted(rec.asset, &burn_acc, rec.encrypted_amount, burn_proof);
            if res2.is_err() {
                println!("burn failed");
                return Err(Error::<T>::NotFound.into());
            }
            Pending::<T>::remove(id);

            Self::deposit_event(Event::OutboundTransferConfirmed {
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
        /// - If called by `XcmOrigin` at any time, refund escrow → sender.
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

            let rec = Pending::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(!rec.completed, Error::<T>::AlreadyCompleted);

            // Two options for authority:
            // 1) Original sender *after* deadline.
            // 2) Privileged confirm origin (e.g., an XCM admin) at any time.
            if let Ok(who) = ensure_signed(caller.clone()) {
                ensure!(who == rec.from, Error::<T>::NotSender);
                let now = <frame_system::Pallet<T>>::block_number();
                ensure!(now >= rec.deadline, Error::<T>::NotExpired);
            } else {
                // If not signed, require the confirm origin.
                T::XcmOrigin::ensure_origin(caller)?;
            }

            // Refund escrow → original sender.
            T::Escrow::escrow_refund(rec.asset, &rec.from, rec.encrypted_amount, refund_proof)
                .map_err(|_| Error::<T>::BackendError)?;
            Pending::<T>::remove(id);

            Self::deposit_event(Event::OutboundTransferRefunded {
                id,
                asset: rec.asset,
            });
            Ok(())
        }

        /// Optimistically handle incoming confidential transfers from sibling parachains.
        /// THIS IS INTENTIONALLY UNSAFE FOR DEMO PURPOSES DO NOT USE IN PRODUCTION
        /// Called automatically when an XCM Transact arrives with
        /// `RuntimeCall::ConfidentialBridge::on_incoming_packet`.
        #[pallet::call_index(3)] // just ensure unique index
        #[pallet::weight(T::WeightInfo::cancel_and_refund())]
        pub fn receive_confidential(
            origin: T::RuntimeOrigin,
            payload: BoundedVec<u8, T::MaxBridgePayload>, //make constant MAX_BRIDGE_PAYLOAD = 1024
        ) -> DispatchResult {
            T::XcmOrigin::ensure_origin(origin)?;

            // Decode the BridgePacket
            let packet: BridgePacket<T::AccountId, T::AssetId> =
                parity_scale_codec::Decode::decode(&mut &payload[..])
                    .map_err(|_| Error::<T>::BackendError)?;
            // Mint encrypted balance locally
            let minted = T::Backend::mint_encrypted(
                packet.asset,
                &packet.dest_account,
                packet.accept_envelope,
            )?;

            Self::deposit_event(Event::InboundTransferExecuted {
                id: packet.transfer_id,
                asset: packet.asset,
                minted,
            });

            Ok(())
        }
    }
}
