// pallets/confidential-intents-dex/src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;

use confidential_assets_primitives::{ConfidentialSwapIntents, EncryptedAmount, InputProof};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[derive(
        Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, sp_runtime::RuntimeDebug,
    )]
    pub struct DexIntent<AccountId, AssetId> {
        pub maker: AccountId,
        pub asset_a: AssetId,
        pub asset_b: AssetId,
        pub a_to_b_ct: EncryptedAmount,
        pub a_to_b_proof: InputProof,
        pub terms_hash: Option<[u8; 32]>, // None => accept any taker ciphertext on asset_b
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type AssetId: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo;
        /// Kept because some upstream traits still carry a Balance type; unused here.
        type Balance: Parameter + Member + Copy + Ord + MaxEncodedLen + TypeInfo + Default;

        /// The opaque identifier used by the Swaps pallet for an opened intent.
        type SwapId: Parameter
            + Member
            + Copy
            + Clone
            + Eq
            + PartialEq
            + MaxEncodedLen
            + TypeInfo
            + core::fmt::Debug;

        /// A swaps pallet that implements `ConfidentialSwapIntents` (open/execute/cancel),
        /// and whose `SwapId` matches `Self::SwapId`.
        type Swaps: ConfidentialSwapIntents<Self::AccountId, Self::AssetId, SwapId = Self::SwapId>;

        type WeightInfo: WeightInfo;
    }

    pub trait WeightInfo {
        fn open_intent() -> Weight;
        fn cancel_intent() -> Weight;
        fn match_intent() -> Weight;
    }
    impl WeightInfo for () {
        fn open_intent() -> Weight {
            10_000.into()
        }
        fn cancel_intent() -> Weight {
            5_000.into()
        }
        fn match_intent() -> Weight {
            30_000.into()
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage
    #[pallet::storage]
    #[pallet::getter(fn next_id)]
    pub type NextId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn intents)]
    pub type Intents<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, DexIntent<T::AccountId, T::AssetId>, OptionQuery>;

    // Events / Errors
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        IntentOpened {
            id: u64,
            maker: T::AccountId,
            asset_a: T::AssetId,
            asset_b: T::AssetId,
        },
        IntentCanceled {
            id: u64,
            maker: T::AccountId,
        },
        IntentMatched {
            id: u64,
            maker: T::AccountId,
            taker: T::AccountId,
            swap_id: T::SwapId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownIntent,
        NotMaker,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Maker posts an *open* confidential intent (no counterparty yet).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::open_intent())]
        pub fn open_intent(
            origin: OriginFor<T>,
            asset_a: T::AssetId,
            asset_b: T::AssetId,
            a_to_b_ct: EncryptedAmount,
            a_to_b_proof: InputProof,
            terms_hash: Option<[u8; 32]>,
        ) -> DispatchResult {
            let maker = ensure_signed(origin)?;
            let id = NextId::<T>::mutate(|n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });

            Intents::<T>::insert(
                id,
                DexIntent {
                    maker: maker.clone(),
                    asset_a,
                    asset_b,
                    a_to_b_ct,
                    a_to_b_proof,
                    terms_hash,
                },
            );

            <Pallet<T>>::deposit_event(Event::IntentOpened {
                id,
                maker,
                asset_a,
                asset_b,
            });
            Ok(())
        }

        /// Maker cancels their open intent.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::cancel_intent())]
        pub fn cancel_intent(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let intent = Intents::<T>::take(id).ok_or(Error::<T>::UnknownIntent)?;
            ensure!(intent.maker == who, Error::<T>::NotMaker);
            <Pallet<T>>::deposit_event(Event::IntentCanceled { id, maker: who });
            Ok(())
        }

        /// Taker matches an intent by supplying their ciphertext leg.
        ///
        /// Flow:
        ///  1) DEX binds the taker by opening a concrete swap in the Swaps pallet.
        ///  2) DEX immediately calls `execute_intent_cc` to execute atomically.
        ///  3) Emits an event with the swap id.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::match_intent())]
        #[transactional]
        pub fn match_intent(
            origin: OriginFor<T>,
            id: u64,
            b_to_a_ct: EncryptedAmount,
            b_to_a_proof: InputProof,
        ) -> DispatchResult {
            let taker = ensure_signed(origin)?;
            let intent = Intents::<T>::take(id).ok_or(Error::<T>::UnknownIntent)?;

            // 1) Bind taker & open a concrete swap in Swaps pallet
            let swap_id: T::SwapId = <T as Config>::Swaps::open_intent_cc(
                &intent.maker,
                &taker,
                intent.asset_a,
                intent.asset_b,
                intent.a_to_b_ct.clone(),
                intent.a_to_b_proof.clone(),
                intent.terms_hash,
            )?;

            // 2) Execute the swap on behalf of the taker (SwapId is Copy; no move issues)
            let _ =
                <T as Config>::Swaps::execute_intent_cc(&taker, swap_id, b_to_a_ct, b_to_a_proof)?;

            // 3) Emit
            <Pallet<T>>::deposit_event(Event::IntentMatched {
                id,
                maker: intent.maker,
                taker,
                swap_id,
            });

            Ok(())
        }
    }
}
