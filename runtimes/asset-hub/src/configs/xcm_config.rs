use super::TransactionByteFee;
use crate::{
    AccountId, AllPalletsWithSystem, AssetId, Assets, Balance, Balances, ForeignAssets,
    ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent,
    RuntimeHoldReason, RuntimeOrigin, WeightToFee, XcmpQueue,
};
use frame_support::{
    parameter_types,
    traits::{
        fungible::HoldConsideration, ConstU32, Contains, Everything, LinearStoragePrice, Nothing,
    },
    weights::Weight,
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_common::impls::ToAuthor;
use polkadot_sdk::{
    polkadot_sdk_frame::runtime::prelude::Identity, staging_xcm as xcm,
    staging_xcm_builder as xcm_builder, staging_xcm_executor as xcm_executor, *,
};
use sp_runtime::traits::TryConvertInto;
use xcm::latest::{prelude::*, WESTEND_GENESIS_HASH};
use xcm_builder::{
    AccountId32Aliases, AllowHrmpNotificationsFromRelayChain, AllowKnownQueryResponses,
    AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom, DescribeAllTerminal, DescribeFamily,
    EnsureXcmOrigin, ExternalConsensusLocationsConverterFor, FixedWeightBounds,
    FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter, HashedDescription, IsConcrete,
    LocalMint, NativeAsset, NoChecking, ParentIsPreset, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, StartsWith, TakeWeightCredit, TrailingSetTopicAsId,
    UsingComponents, WithComputedOrigin, WithUniqueTopic,
};
use xcm_executor::XcmExecutor;

// bring in the pallet instances defined in configs/mod.rs
use super::{ForeignAssetsInstance, TrustBackedAssetsInstance};

// glue traits & helpers missing in original
use crate::{Authorship, CollatorSelection};
use polkadot_sdk::polkadot_sdk_frame::traits::{EverythingBut, PalletInfoAccess};

// Weight unit price used by FixedWeightBounds
parameter_types! {
    pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 0);
}

parameter_types! {
    pub const RelayLocation: Location = Location::parent();
    // Local native currency which is stored in `pallet_balances`
    pub const NativeCurrency: Location = Location::here();
    // This runtime is used for testing; default network is Westend by genesis hash.
    pub storage RelayNetworkId: NetworkId = NetworkId::ByGenesis(WESTEND_GENESIS_HASH);
    pub RelayNetwork: Option<NetworkId> = Some(RelayNetworkId::get());
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorLocation = [
        GlobalConsensus(RelayNetworkId::get()),
        Parachain(ParachainInfo::parachain_id().into())
    ].into();
    pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();
    pub StakingPot: AccountId = CollatorSelection::account_id();
    pub ForeignAssetsPalletLocation: Location =
        PalletInstance(<ForeignAssets as PalletInfoAccess>::index() as u8).into();
    pub TrustBackedAssetsPalletIndex: u8 = <Assets as PalletInfoAccess>::index() as u8;
    pub TrustBackedAssetsPalletLocation: Location =
        PalletInstance(TrustBackedAssetsPalletIndex::get()).into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Map `Location` → `AccountId`
pub type LocationToAccountId = (
    ParentIsPreset<AccountId>,
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<RelayNetwork, AccountId>,
    HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
    ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>,
);

/// Native balances transactor
pub type FungibleTransactor =
    FungibleAdapter<Balances, IsConcrete<NativeCurrency>, LocationToAccountId, AccountId, ()>;

/// `AssetId`/`Balance` converter for trust-backed (Instance1) assets
pub type TrustBackedAssetsConvertedConcreteId =
    assets_common::TrustBackedAssetsConvertedConcreteId<TrustBackedAssetsPalletLocation, Balance>;

/// Pallet-assets (Instance1) transactor
pub type FungiblesTransactor = FungiblesAdapter<
    Assets,
    TrustBackedAssetsConvertedConcreteId,
    LocationToAccountId,
    AccountId,
    LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, Assets>>,
    CheckingAccount,
>;

// Using latest `Location`
pub type ForeignAssetsAssetId = Location;

/// Matched converter for foreign-assets (Instance2)
pub type ForeignAssetsConvertedConcreteId = xcm_builder::MatchedConvertedConcreteId<
    Location,
    Balance,
    EverythingBut<(StartsWith<assets_common::matching::LocalLocationPattern>,)>,
    Identity,
    TryConvertInto,
>;

/// Pallet-assets (Instance2) transactor
pub type ForeignFungiblesTransactor = FungiblesAdapter<
    ForeignAssets,
    ForeignAssetsConvertedConcreteId,
    LocationToAccountId,
    AccountId,
    NoChecking,
    CheckingAccount,
>;

// /// `AssetId`/`Balance` converter for pool assets (Instance3)
// pub type PoolAssetsConvertedConcreteId =
//     assets_common::PoolAssetsConvertedConcreteId<PoolAssetsPalletLocation, Balance>;

// /// Pallet-assets (Instance3) transactor
// pub type PoolFungiblesTransactor = FungiblesAdapter<
//     PoolAssets,
//     PoolAssetsConvertedConcreteId,
//     LocationToAccountId,
//     AccountId,
//     LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, PoolAssets>>,
//     CheckingAccount,
// >;

/// Union transactor set
pub type AssetTransactors = (
    FungibleTransactor,
    FungiblesTransactor,
    ForeignFungiblesTransactor,
    // PoolFungiblesTransactor,
);

/// XCM origin → local origin for `Transact`
pub type XcmOriginToTransactDispatchOrigin = (
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
    pub XcmAssetFeesReceiver: Option<AccountId> = Authorship::author();

    // Replace Westend-only `deposit()` helper with local constants
    pub const DepositPerItem: Balance = 10 * crate::MICRO_UNIT;
    pub const DepositPerByte: Balance = 1 * crate::MICRO_UNIT;
    pub const AuthorizeAliasHoldReason: RuntimeHoldReason =
        RuntimeHoldReason::PolkadotXcm(pallet_xcm::HoldReason::AuthorizeAlias);

    // Base price (arbitrary but sensible) for UMP delivery pricing
    pub const BaseDeliveryFee: Balance = 10 * crate::MICRO_UNIT;
}

// /// Adapter that can exchange pool assets to the fee asset when paying for execution
// pub type PoolAssetsExchanger = SingleAssetExchangeAdapter<
//     crate::AssetConversion,
//     crate::NativeAndNonPoolAssets,
//     (
//         assets_common::TrustBackedAssetsAsLocation<
//             TrustBackedAssetsPalletLocation,
//             Balance,
//             xcm::latest::Location,
//         >,
//         ForeignAssetsConvertedConcreteId,
//     ),
//     AccountId,
// >;

/// Parent delivery price model (make concrete to avoid generic-type errors)
pub type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
    AssetId,
    BaseDeliveryFee,
    TransactionByteFee,
    ParachainSystem,
>;

pub struct ParentOrParentsExecutivePlurality;
impl Contains<Location> for ParentOrParentsExecutivePlurality {
    fn contains(location: &Location) -> bool {
        matches!(
            location.unpack(),
            (1, [])
                | (
                    1,
                    [Plurality {
                        id: BodyId::Executive,
                        ..
                    }]
                )
        )
    }
}

pub type Barrier = TrailingSetTopicAsId<(
    TakeWeightCredit,
    AllowKnownQueryResponses<PolkadotXcm>,
    WithComputedOrigin<
        (
            AllowTopLevelPaidExecutionFrom<Everything>,
            AllowSubscriptionsFrom<Everything>,
            AllowHrmpNotificationsFromRelayChain,
        ),
        UniversalLocation,
        ConstU32<8>,
    >,
)>;

/// Fee multiplier for trust-backed assets when using TakeFirstAssetTrader
pub type AssetFeeAsExistentialDepositMultiplierFeeCharger =
    parachains_common::xcm_config::AssetFeeAsExistentialDepositMultiplier<
        Runtime,
        WeightToFee,
        pallet_assets::BalanceToAssetBalance<
            Balances,
            Runtime,
            sp_runtime::traits::ConvertInto,
            TrustBackedAssetsInstance,
        >,
        TrustBackedAssetsInstance,
    >;

/// Fee multiplier for foreign assets — define **once** (removed duplicate)
pub type ForeignAssetFeeAsExistentialDepositMultiplierFeeCharger =
    parachains_common::xcm_config::AssetFeeAsExistentialDepositMultiplier<
        Runtime,
        WeightToFee,
        pallet_assets::BalanceToAssetBalance<
            Balances,
            Runtime,
            sp_runtime::traits::ConvertInto,
            ForeignAssetsInstance,
        >,
        ForeignAssetsInstance,
    >;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type XcmEventEmitter = PolkadotXcm;
    // How to withdraw and deposit an asset.
    type AssetTransactor = FungibleTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = (); // Teleporting is disabled.
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader =
        UsingComponents<WeightToFee, NativeCurrency, AccountId, Balances, ToAuthor<Runtime>>;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type AssetLocker = ();
    type AssetExchanger = (); //PoolAssetsExchanger;
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
    // If you don't maintain a custom alias allowlist, keep this `Nothing`.
    type Aliasers = Nothing;
    type TransactionalProcessor = FrameTransactionalProcessor;
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
    type XcmRecorder = PolkadotXcm;
}

/// Local signed origin → Location (for send/execute)
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// Message routing
pub type XcmRouter = WithUniqueTopic<(
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>,
    XcmpQueue,
)>;

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<8>;
    type WeightInfo = pallet_xcm::TestWeightInfo;
    type AdminOrigin = EnsureRoot<AccountId>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AuthorizedAliasConsideration = HoldConsideration<
        AccountId,
        Balances,
        AuthorizeAliasHoldReason,
        LinearStoragePrice<DepositPerItem, DepositPerByte, Balance>,
    >;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<ForeignAssetsAssetId> for XcmBenchmarkHelper {
    fn create_asset_id_parameter(id: u32) -> ForeignAssetsAssetId {
        Location::new(1, [Parachain(id)])
    }
}
