// pallets/confidential-htlc/src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;

use confidential_assets_primitives::{
    AdaptorSigBackend, BridgeHtlc, EncryptedAmount, EscrowTrust, InputProof,
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The concrete parameter the escrow expects: (ciphertext delta, proof).
    pub type EscrowParam = (EncryptedAmount, InputProof);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type AssetId: Parameter + MaxEncodedLen + TypeInfo + Copy + Ord;

        /// Escrow movement — expects (EncryptedAmount, InputProof).
        type Escrow: EscrowTrust<Self::AccountId, Self::AssetId, EscrowParam>;

        /// Crypto for hashlock + adaptor-signature math.
        type Crypto: AdaptorSigBackend;

        type WeightInfo: WeightInfo;
    }

    pub trait WeightInfo {
        fn open_htlc() -> Weight;
        fn redeem_with_secret() -> Weight;
        fn redeem_with_adaptor_sig() -> Weight;
        fn refund() -> Weight;
    }
    impl WeightInfo for () {
        fn open_htlc() -> Weight {
            Weight::from_parts(20_000, 0)
        }
        fn redeem_with_secret() -> Weight {
            Weight::from_parts(25_000, 0)
        }
        fn redeem_with_adaptor_sig() -> Weight {
            Weight::from_parts(25_000, 0)
        }
        fn refund() -> Weight {
            Weight::from_parts(30_000, 0)
        }
    }

    // ---------------------------
    // Types & Storage
    // ---------------------------

    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, RuntimeDebug)]
    pub enum HtlcState {
        Open,
        Redeemed,
        Refunded,
    }

    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, RuntimeDebug)]
    pub struct Htlc<AccountId, AssetId, BlockNumber, HashLock> {
        pub maker: AccountId,
        pub taker: Option<AccountId>,
        pub asset: AssetId,
        pub param: EscrowParam, // (EncryptedAmount, InputProof)
        pub hashlock: HashLock,
        pub expiry: BlockNumber,
        pub adaptor_partial: Option<BoundedVec<u8, ConstU32<64>>>, // 64 bytes expected (opaque)
        pub state: HtlcState,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Monotonic HTLC id counter.
    #[pallet::storage]
    pub(super) type NextId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// htlc_id -> record
    #[pallet::storage]
    pub(super) type Htlcs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        Htlc<
            T::AccountId,
            T::AssetId,
            BlockNumberFor<T>,
            <T::Crypto as AdaptorSigBackend>::HashLock,
        >,
        OptionQuery,
    >;

    // ---------------------------
    // Events / Errors
    // ---------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        HtlcOpened {
            id: u64,
            maker: T::AccountId,
            taker: Option<T::AccountId>,
            asset: T::AssetId,
            param: EscrowParam,
            expiry: BlockNumberFor<T>,
        },
        HtlcRedeemed {
            id: u64,
            redeemer: T::AccountId,
            secret: Vec<u8>,
        },
        HtlcRefunded {
            id: u64,
            maker: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
        NotOpen,
        NotAuthorized,
        NotYetExpired,
        BadSecret,
        BadSignature,
        Arithmetic,
        MalformedSignature,
    }

    impl<T: Config> Pallet<T> {
        #[inline]
        fn vec_to_array_64(bytes: &[u8]) -> Result<[u8; 64], Error<T>> {
            if bytes.len() != 64 {
                return Err(Error::<T>::MalformedSignature);
            }
            let mut arr = [0u8; 64];
            arr.copy_from_slice(bytes);
            Ok(arr)
        }
    }

    // ---------------------------
    // Calls (extrinsics)
    // ---------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Maker opens + funds an HTLC. Escrows the (Δ, proof).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::open_htlc())]
        pub fn open_htlc(
            origin: OriginFor<T>,
            taker: Option<T::AccountId>,
            asset: T::AssetId,
            delta: EncryptedAmount,
            proof: InputProof,
            hashlock: <T::Crypto as AdaptorSigBackend>::HashLock,
            expiry: BlockNumberFor<T>,
            adaptor_partial: Option<Vec<u8>>,
        ) -> DispatchResult {
            let maker = ensure_signed(origin)?;
            let param: EscrowParam = (delta, proof);

            // Lock into escrow
            T::Escrow::escrow_lock(asset, &maker, param.clone())
                .map_err(|_| Error::<T>::Arithmetic)?;

            // Store HTLC
            let id = NextId::<T>::mutate(|x| {
                let id = *x;
                *x = x.saturating_add(1);
                id
            });

            // Bound adaptor bytes (if present)
            let adaptor_bounded: Option<BoundedVec<_, ConstU32<64>>> = match adaptor_partial {
                Some(bytes) => Some(bytes.try_into().map_err(|_| Error::<T>::Arithmetic)?),
                None => None,
            };

            let rec = Htlc::<T::AccountId, T::AssetId, BlockNumberFor<T>, _> {
                maker: maker.clone(),
                taker,
                asset,
                param: param.clone(),
                hashlock,
                expiry,
                adaptor_partial: adaptor_bounded,
                state: HtlcState::Open,
            };
            Htlcs::<T>::insert(id, rec);

            let taker_for_event = Htlcs::<T>::get(id).and_then(|r| r.taker);
            Self::deposit_event(Event::HtlcOpened {
                id,
                maker,
                taker: taker_for_event,
                asset,
                param,
                expiry,
            });
            Ok(())
        }

        /// Redeem with preimage `secret`. `who` must be the taker if specified, else anyone presenting the valid secret.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::redeem_with_secret())]
        pub fn redeem_with_secret(
            origin: OriginFor<T>,
            htlc_id: u64,
            secret: <T::Crypto as AdaptorSigBackend>::Secret,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);
            if let Some(taker) = &rec.taker {
                ensure!(&who == taker, Error::<T>::NotAuthorized);
            }

            // Check hashlock
            let h = <T::Crypto as AdaptorSigBackend>::hash_secret(&secret);
            ensure!(h == rec.hashlock, Error::<T>::BadSecret);

            // Release escrow to taker (or to `who`)
            let to = rec.taker.as_ref().unwrap_or(&who);
            T::Escrow::escrow_release(rec.asset, to, rec.param.clone())
                .map_err(|_| Error::<T>::Arithmetic)?;

            rec.state = HtlcState::Redeemed;
            Htlcs::<T>::insert(htlc_id, &rec);

            // Emit secret bytes so the *other* chain can learn it (bridge watches this)
            let secret_bytes = secret.encode();
            Self::deposit_event(Event::HtlcRedeemed {
                id: htlc_id,
                redeemer: who,
                secret: secret_bytes,
            });
            Ok(())
        }

        /// Redeem with final signature. Pallet recovers the secret using (partial, final),
        /// verifies hashlock, releases escrow, and emits the recovered secret.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::redeem_with_adaptor_sig())]
        pub fn redeem_with_adaptor_sig(
            origin: OriginFor<T>,
            htlc_id: u64,
            final_sig: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);
            if let Some(taker) = &rec.taker {
                ensure!(&who == taker, Error::<T>::NotAuthorized);
            }

            // Get the stored adaptor partial (must be 64 bytes)
            let partial_vec = rec
                .adaptor_partial
                .clone()
                .ok_or(Error::<T>::BadSignature)?;
            let partial_arr =
                Self::vec_to_array_64(&partial_vec).map_err(|_| Error::<T>::BadSignature)?;

            // Final signature must be 64 bytes
            let final_arr =
                Self::vec_to_array_64(&final_sig).map_err(|_| Error::<T>::BadSignature)?;

            // Recover secret
            let secret = <T::Crypto as AdaptorSigBackend>::recover_secret_from_sigs(
                &partial_arr,
                &final_arr,
            )
            .map_err(|_| Error::<T>::BadSignature)?;

            // Check hashlock
            let h = <T::Crypto as AdaptorSigBackend>::hash_secret(&secret);
            ensure!(h == rec.hashlock, Error::<T>::BadSecret);

            // Release escrow to taker (or `who`)
            let to = rec.taker.as_ref().unwrap_or(&who);
            T::Escrow::escrow_release(rec.asset, to, rec.param.clone())
                .map_err(|_| Error::<T>::Arithmetic)?;

            rec.state = HtlcState::Redeemed;
            Htlcs::<T>::insert(htlc_id, &rec);

            let secret_bytes = secret.encode();
            Self::deposit_event(Event::HtlcRedeemed {
                id: htlc_id,
                redeemer: who,
                secret: secret_bytes,
            });
            Ok(())
        }

        /// Refund to maker after expiry.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::refund())]
        pub fn refund(origin: OriginFor<T>, htlc_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);
            ensure!(who == rec.maker, Error::<T>::NotAuthorized);
            ensure!(
                frame_system::Pallet::<T>::block_number() >= rec.expiry,
                Error::<T>::NotYetExpired
            );

            T::Escrow::escrow_refund(rec.asset, &rec.maker, rec.param.clone())
                .map_err(|_| Error::<T>::Arithmetic)?;

            rec.state = HtlcState::Refunded;
            Htlcs::<T>::insert(htlc_id, &rec);

            Self::deposit_event(Event::HtlcRefunded {
                id: htlc_id,
                maker: who,
            });
            Ok(())
        }
    }

    // ---------------------------
    // BridgeHtlc impl (for your bridge pallet)
    // ---------------------------

    impl<T: Config> BridgeHtlc<T::AccountId, T::AssetId, EscrowParam> for Pallet<T> {
        type HashLock = <T::Crypto as AdaptorSigBackend>::HashLock;
        type Secret = <T::Crypto as AdaptorSigBackend>::Secret;

        fn open_htlc(
            maker: &T::AccountId,
            taker: Option<T::AccountId>,
            asset: T::AssetId,
            amount: EscrowParam,
            hashlock: Self::HashLock,
            expiry_abs: u32,
            adaptor_partial: Option<Vec<u8>>,
        ) -> Result<u64, DispatchError> {
            // Lock into escrow
            T::Escrow::escrow_lock(asset, maker, amount.clone())?;

            let id = NextId::<T>::mutate(|x| {
                let id = *x;
                *x = x.saturating_add(1);
                id
            });
            let expiry_bn: BlockNumberFor<T> = expiry_abs.into();

            let adaptor_bounded: Option<BoundedVec<_, ConstU32<64>>> = match adaptor_partial {
                Some(bytes) => Some(bytes.try_into().map_err(|_| Error::<T>::Arithmetic)?),
                None => None,
            };

            let rec = Htlc::<T::AccountId, T::AssetId, BlockNumberFor<T>, _> {
                maker: maker.clone(),
                taker,
                asset,
                param: amount,
                hashlock,
                expiry: expiry_bn,
                adaptor_partial: adaptor_bounded,
                state: HtlcState::Open,
            };
            Htlcs::<T>::insert(id, rec);
            Ok(id)
        }

        fn redeem_with_secret(
            who: &T::AccountId,
            htlc_id: u64,
            secret: Self::Secret,
        ) -> Result<Self::Secret, DispatchError> {
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            if let Some(taker) = &rec.taker {
                ensure!(who == taker, Error::<T>::NotAuthorized);
            }
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);
            ensure!(
                <T::Crypto as AdaptorSigBackend>::hash_secret(&secret) == rec.hashlock,
                Error::<T>::BadSecret
            );

            let to = rec.taker.as_ref().unwrap_or(who);
            T::Escrow::escrow_release(rec.asset, to, rec.param.clone())?;
            rec.state = HtlcState::Redeemed;
            Htlcs::<T>::insert(htlc_id, &rec);
            Ok(secret)
        }

        fn redeem_with_adaptor_sig(
            who: &T::AccountId,
            htlc_id: u64,
            final_sig: Vec<u8>,
        ) -> Result<Self::Secret, DispatchError> {
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            if let Some(taker) = &rec.taker {
                ensure!(who == taker, Error::<T>::NotAuthorized);
            }
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);

            let partial_vec = rec
                .adaptor_partial
                .clone()
                .ok_or(Error::<T>::BadSignature)?;
            let partial_arr =
                Pallet::<T>::vec_to_array_64(&partial_vec).map_err(|_| Error::<T>::BadSignature)?;
            let final_arr =
                Pallet::<T>::vec_to_array_64(&final_sig).map_err(|_| Error::<T>::BadSignature)?;

            let secret = <T::Crypto as AdaptorSigBackend>::recover_secret_from_sigs(
                &partial_arr,
                &final_arr,
            )?;
            ensure!(
                <T::Crypto as AdaptorSigBackend>::hash_secret(&secret) == rec.hashlock,
                Error::<T>::BadSecret
            );

            let to = rec.taker.as_ref().unwrap_or(who);
            T::Escrow::escrow_release(rec.asset, to, rec.param.clone())?;
            rec.state = HtlcState::Redeemed;
            Htlcs::<T>::insert(htlc_id, &rec);
            Ok(secret)
        }

        fn refund(who: &T::AccountId, htlc_id: u64) -> DispatchResult {
            let mut rec = Htlcs::<T>::get(htlc_id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(rec.state, HtlcState::Open), Error::<T>::NotOpen);
            ensure!(who == &rec.maker, Error::<T>::NotAuthorized);
            ensure!(
                frame_system::Pallet::<T>::block_number() >= rec.expiry,
                Error::<T>::NotYetExpired
            );
            T::Escrow::escrow_refund(rec.asset, &rec.maker, rec.param.clone())?;
            rec.state = HtlcState::Refunded;
            Htlcs::<T>::insert(htlc_id, &rec);
            Ok(())
        }
    }
}
