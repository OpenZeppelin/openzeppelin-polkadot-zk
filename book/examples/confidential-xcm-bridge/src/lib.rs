// pallets/confidential-xcm-bridge/src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[frame_support::pallet]
pub mod pallet {
    use confidential_assets_primitives::*;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::{Decode, Encode};
    use scale_info::TypeInfo;
    use sp_std::prelude::*;

    // === Router abstraction (don’t hard-code XCM version or pallet here) ===
    pub trait XcmRouter {
        type ParaId: Parameter + Copy + MaxEncodedLen + TypeInfo;
        type Weight: Parameter + Copy + MaxEncodedLen + TypeInfo + Default;
        type FeeAssetId: Parameter + Copy + MaxEncodedLen + TypeInfo;
        type FeeBalance: Parameter + Copy + MaxEncodedLen + TypeInfo;

        /// Send a SCALE-encoded payload to `dest` via XCM::Transact.
        fn send_transact(
            dest: Self::ParaId,
            payload: Vec<u8>,
            fee_asset: Self::FeeAssetId,
            fee: Self::FeeBalance,
            weight_limit: Self::Weight,
        ) -> Result<(), DispatchError>;
    }

    // === Escrow param used by the confidential HTLC path ===
    pub type EscrowParam = (EncryptedAmount, InputProof);

    // Internal helper alias; never used in public types.
    pub type HtlcSecretOf<T> = <<T as Config>::ConfidentialHtlc as BridgeHtlc<
        <T as frame_system::Config>::AccountId,
        <T as Config>::AssetId,
        EscrowParam,
    >>::Secret;

    // === SCALE payloads carried by XCM::Transact ===
    // IMPORTANT: Do not leak `Secret` in public metadata; use Vec<u8> on the wire.
    #[derive(Encode, Decode, TypeInfo, Clone)]
    pub enum RemoteCall<AccountId, AssetId> {
        /// Credit a confidential transfer on the destination chain.
        ReceiveConfidentialTransfer {
            sender_on_src: [u8; 32],
            dest_account: AccountId,
            asset: AssetId,
            /// Optional audit tag; destination doesn’t need it for mint, but we keep it for tracing.
            delta_ciphertext: EncryptedAmount,
            /// Proof used on the DESTINATION chain to mint the encrypted amount.
            mint_proof: InputProof,
        },
        /// Execute an HTLC redeem-by-secret on the destination chain.
        HtlcRedeemWithSecret {
            who: AccountId,
            htlc_id: u64,
            // SCALE-encoded `Secret`, opaque to this pallet
            secret_bytes: Vec<u8>,
        },
        /// Execute an HTLC redeem-by-adaptor-sig on the destination chain.
        HtlcRedeemWithAdaptorSig {
            who: AccountId,
            htlc_id: u64,
            final_sig: Vec<u8>,
        },
    }

    // === Config ===
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Asset identifier
        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;

        /// Balance value type (unused directly here, kept for symmetry with Backend)
        type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default;

        /// Backend for encrypted balances (used for debit on source, mint on destination).
        type Backend: ConfidentialBackend<Self::AccountId, Self::AssetId, Self::Balance>;

        /// Plug-in ramp (kept for symmetry; unused by this pallet’s calls right now).
        type Ramp: Ramp<Self::AccountId, Self::AssetId, Self::Balance>;

        /// XCM router used to actually send Transact messages.
        type Xcm: XcmRouter<
            ParaId = Self::ParaId,
            Weight = Self::XcmWeight,
            FeeAssetId = Self::FeeAssetId,
            FeeBalance = Self::FeeBalance,
        >;

        /// Confidential HTLC implementation (escrow path).
        /// Must match: BridgeHtlc<AccountId, AssetId, (EncryptedAmount, InputProof)>
        type ConfidentialHtlc: BridgeHtlc<Self::AccountId, Self::AssetId, EscrowParam>;

        /// Concrete types for the router.
        type ParaId: Parameter + Copy + MaxEncodedLen + TypeInfo;
        type XcmWeight: Parameter + Copy + MaxEncodedLen + TypeInfo + Default;
        type FeeAssetId: Parameter + Copy + MaxEncodedLen + TypeInfo;
        type FeeBalance: Parameter + Copy + MaxEncodedLen + TypeInfo;

        type WeightInfo: WeightInfo;
    }

    // === Weights ===
    pub trait WeightInfo {
        fn send_confidential_transfer() -> Weight;
        fn send_htlc_redeem_with_secret() -> Weight;
        fn send_htlc_redeem_with_adaptor_sig() -> Weight;
        fn xcm_handle() -> Weight;
    }
    impl WeightInfo for () {
        fn send_confidential_transfer() -> Weight {
            Weight::from_parts(20_000, 0)
        }
        fn send_htlc_redeem_with_secret() -> Weight {
            Weight::from_parts(25_000, 0)
        }
        fn send_htlc_redeem_with_adaptor_sig() -> Weight {
            Weight::from_parts(25_000, 0)
        }
        fn xcm_handle() -> Weight {
            Weight::from_parts(30_000, 0)
        }
    }

    // === Storage ===
    #[pallet::storage]
    #[pallet::getter(fn next_nonce)]
    pub type NextNonce<T> = StorageValue<_, u64, ValueQuery>;

    // === Events / Errors ===
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        XcmSent {
            nonce: u64,
            dest: T::ParaId,
            payload_hash: [u8; 32],
        },
        XcmConfTransferApplied {
            from_tag: [u8; 32],
            to: T::AccountId,
            asset: T::AssetId,
        },
        XcmHtlcExecuted {
            who: T::AccountId,
            htlc_id: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        RouterError,
        BackendError,
        HtlcFailed,
        BadOriginForXcm, // replace with EnsureXcm/AuthorizedXcm origin in runtime
        DecodeError,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // === Calls ===
    // NOTE: Put the bound here so the pallet macro can derive metadata for Call.
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        HtlcSecretOf<T>: Decode,
    {
        /// Source-chain: confidential cross-chain send **with local debit**.
        ///
        /// This mirrors `withdraw` in `confidential-assets`:
        /// 1) Locally **burn** (debit) the encrypted balance using `burn_proof` (fails if insufficient).
        /// 2) XCM a payload that the destination uses to **mint** via `mint_proof`.
        ///
        /// We split proofs so the destination mint witness is not consumed by the source burn.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::send_confidential_transfer())]
        pub fn send_confidential_transfer(
            origin: OriginFor<T>,
            dest: T::ParaId,
            sender_tag: [u8; 32],
            beneficiary: T::AccountId,
            asset: T::AssetId,
            // encrypted delta being moved cross-chain
            delta_ciphertext: EncryptedAmount,
            // proof for local **burn** (debit on SOURCE)
            burn_proof: InputProof,
            // proof to use on DESTINATION to **mint**
            mint_proof: InputProof,
            fee_asset: T::FeeAssetId,
            fee: T::FeeBalance,
            weight_limit: T::XcmWeight,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1) Locally debit confidential balance. Fails if insufficient.
            //    We ignore the returned plaintext `amount` to preserve privacy here.
            let _amount =
                T::Backend::burn_encrypted(asset, &who, delta_ciphertext.clone(), burn_proof)
                    .map_err(|_| Error::<T>::BackendError)?;

            // 2) Ship the DEST mint proof to the destination chain.
            let call = RemoteCall::<T::AccountId, T::AssetId>::ReceiveConfidentialTransfer {
                sender_on_src: sender_tag,
                dest_account: beneficiary,
                asset,
                delta_ciphertext, // kept for auditability
                mint_proof,
            };
            let payload = Encode::encode(&call);
            let payload_hash = sp_io::hashing::blake2_256(&payload);

            T::Xcm::send_transact(dest, payload, fee_asset, fee, weight_limit)
                .map_err(|_| Error::<T>::RouterError)?;

            let nonce = NextNonce::<T>::mutate(|n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });

            Self::deposit_event(Event::XcmSent {
                nonce,
                dest,
                payload_hash,
            });
            Ok(())
        }

        /// Source-chain: relay an HTLC preimage to the destination chain for atomic redemption there.
        /// Assumes funds were already escrowed locally via your `pallet-confidential-htlc::open_htlc`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::send_htlc_redeem_with_secret())]
        pub fn send_htlc_redeem_with_secret(
            origin: OriginFor<T>,
            dest: T::ParaId,
            who: T::AccountId,
            htlc_id: u64,
            secret_bytes: Vec<u8>, // opaque wire type
            fee_asset: T::FeeAssetId,
            fee: T::FeeBalance,
            weight_limit: T::XcmWeight,
        ) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            let call = RemoteCall::<T::AccountId, T::AssetId>::HtlcRedeemWithSecret {
                who,
                htlc_id,
                secret_bytes,
            };
            let payload = Encode::encode(&call);
            let payload_hash = sp_io::hashing::blake2_256(&payload);

            T::Xcm::send_transact(dest, payload, fee_asset, fee, weight_limit)
                .map_err(|_| Error::<T>::RouterError)?;

            let nonce = NextNonce::<T>::mutate(|n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });

            Self::deposit_event(Event::XcmSent {
                nonce,
                dest,
                payload_hash,
            });
            Ok(())
        }

        /// Source-chain: relay an HTLC final signature (adaptor flow) to redeem on destination.
        /// Assumes funds were already escrowed locally via your `pallet-confidential-htlc::open_htlc`.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::send_htlc_redeem_with_adaptor_sig())]
        pub fn send_htlc_redeem_with_adaptor_sig(
            origin: OriginFor<T>,
            dest: T::ParaId,
            who: T::AccountId,
            htlc_id: u64,
            final_sig: Vec<u8>,
            fee_asset: T::FeeAssetId,
            fee: T::FeeBalance,
            weight_limit: T::XcmWeight,
        ) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            let call = RemoteCall::<T::AccountId, T::AssetId>::HtlcRedeemWithAdaptorSig {
                who,
                htlc_id,
                final_sig,
            };
            let payload = Encode::encode(&call);
            let payload_hash = sp_io::hashing::blake2_256(&payload);

            T::Xcm::send_transact(dest, payload, fee_asset, fee, weight_limit)
                .map_err(|_| Error::<T>::RouterError)?;

            let nonce = NextNonce::<T>::mutate(|n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });

            Self::deposit_event(Event::XcmSent {
                nonce,
                dest,
                payload_hash,
            });
            Ok(())
        }

        // -------- Inbound handler (gate with EnsureXcm in runtime) --------

        /// Destination-chain: handle inbound XCM payloads.
        /// NOTE: Use a proper XCM origin (EnsureXcm/AuthorizedXcm) in your runtime; Root here is a placeholder.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::xcm_handle())]
        pub fn xcm_handle(origin: OriginFor<T>, payload: Vec<u8>) -> DispatchResult {
            ensure_root(origin).map_err(|_| Error::<T>::BadOriginForXcm)?;

            let call: RemoteCall<T::AccountId, T::AssetId> =
                Decode::decode(&mut &payload[..]).map_err(|_| Error::<T>::DecodeError)?;

            match call {
                RemoteCall::ReceiveConfidentialTransfer {
                    sender_on_src,
                    dest_account,
                    asset,
                    delta_ciphertext: _delta, // present for auditability; backend uses `mint_proof` as truth
                    mint_proof,
                } => {
                    // Mint on destination using the mint witness provided by the source user.
                    T::Backend::mint_encrypted(asset, &dest_account, mint_proof)
                        .map_err(|_| Error::<T>::BackendError)?;
                    Self::deposit_event(Event::XcmConfTransferApplied {
                        from_tag: sender_on_src,
                        to: dest_account,
                        asset,
                    });
                }
                RemoteCall::HtlcRedeemWithSecret {
                    who,
                    htlc_id,
                    secret_bytes,
                } => {
                    // Decode SCALE-encoded Secret locally without leaking it to metadata.
                    let secret: HtlcSecretOf<T> = {
                        let mut cur = &secret_bytes[..];
                        Decode::decode(&mut cur).map_err(|_| Error::<T>::DecodeError)?
                    };

                    <T::ConfidentialHtlc as BridgeHtlc<
                        T::AccountId,
                        T::AssetId,
                        EscrowParam,
                    >>::redeem_with_secret(&who, htlc_id, secret)
                        .map_err(|_| Error::<T>::HtlcFailed)?;
                    Self::deposit_event(Event::XcmHtlcExecuted { who, htlc_id });
                }
                RemoteCall::HtlcRedeemWithAdaptorSig {
                    who,
                    htlc_id,
                    final_sig,
                } => {
                    <T::ConfidentialHtlc as BridgeHtlc<
                        T::AccountId,
                        T::AssetId,
                        EscrowParam,
                    >>::redeem_with_adaptor_sig(&who, htlc_id, final_sig)
                        .map_err(|_| Error::<T>::HtlcFailed)?;
                    Self::deposit_event(Event::XcmHtlcExecuted { who, htlc_id });
                }
            }

            Ok(())
        }
    }
}
