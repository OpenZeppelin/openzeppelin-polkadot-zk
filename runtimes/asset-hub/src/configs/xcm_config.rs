use super::TransactionByteFee;
use crate::{
    AccountId, AllPalletsWithSystem, AssetConversion, AssetId, Assets, Balance, Balances,
    ForeignAssets, ParachainInfo, ParachainSystem, PolkadotXcm, PoolAssets, Runtime, RuntimeCall,
    RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, WeightToFee, XcmpQueue,
};
use polkadot_sdk::{
    staging_xcm as xcm, staging_xcm_builder as xcm_builder, staging_xcm_executor as xcm_executor, *,
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
    polkadot_sdk_frame::traits::Disabled,
    staging_xcm_builder::{DenyRecursively, DenyThenTry},
};
use testnet_parachains_constants::westend::currency::deposit;
use xcm::latest::{prelude::*, WESTEND_GENESIS_HASH};
use xcm_builder::{
    AccountId32Aliases, AliasChildLocation, AliasOriginRootUsingFilter,
    AllowHrmpNotificationsFromRelayChain, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AsPrefixedGeneralIndex, ConvertedConcreteId,
    DescribeAllTerminal, DescribeFamily, DescribeTerminus, EnsureXcmOrigin,
    ExternalConsensusLocationsConverterFor, FixedWeightBounds, FrameTransactionalProcessor,
    FungibleAdapter, FungiblesAdapter, HashedDescription, IsConcrete, LocalMint, NativeAsset,
    NoChecking, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SendXcmFeeToAccount,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SingleAssetExchangeAdapter, SovereignSignedViaLocation, StartsWith,
    TakeWeightCredit, TrailingSetTopicAsId, UsingComponents, WithComputedOrigin, WithUniqueTopic,
    XcmFeeManagerFromComponents,
};
use xcm_executor::{traits::JustTry, XcmExecutor};

parameter_types! {
    pub const RelayLocation: Location = Location::parent();
    // Local native currency which is stored in `pallet_balances`
    pub const NativeCurrency: Location = Location::here();
    // This runtime is utilized for testing with various environment setups.
    // This storage item allows us to customize the `NetworkId` where the runtime is deployed.
    // By default, it is set to `Westend Network` and can be changed using `System::set_storage`.
    pub storage RelayNetworkId: NetworkId = NetworkId::ByGenesis(WESTEND_GENESIS_HASH);
    pub RelayNetwork: Option<NetworkId> = Some(RelayNetworkId::get());
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorLocation = [
        GlobalConsensus(RelayNetworkId::get()),
        Parachain(ParachainInfo::parachain_id().into())
    ].into();
    pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();
    pub TreasuryAccount: AccountId = TREASURY_PALLET_ID.into_account_truncating();
    pub StakingPot: AccountId = CollatorSelection::account_id();
    pub ForeignAssetsPalletLocation: Location =
        PalletInstance(<ForeignAssets as PalletInfoAccess>::index() as u8).into();
    pub PoolAssetsPalletLocation: Location =
        PalletInstance(<PoolAssets as PalletInfoAccess>::index() as u8).into();
    pub TrustBackedAssetsPalletIndex: u8 = <Assets as PalletInfoAccess>::index() as u8;
    pub TrustBackedAssetsPalletLocation: Location =
        PalletInstance(TrustBackedAssetsPalletIndex::get()).into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the parent `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Foreign locations alias into accounts according to a hash of their standard description.
    HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
    // Different global consensus locations sovereign accounts.
    ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>,
);

/// Means for transacting assets on this chain.
pub type FungibleTransactor = FungibleAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<NativeCurrency>,
    // Do a simple punn to convert an AccountId32 Location into a native chain account ID:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports.
    (),
>;

/// `AssetId`/`Balance` converter for `TrustBackedAssets`.
pub type TrustBackedAssetsConvertedConcreteId =
    assets_common::TrustBackedAssetsConvertedConcreteId<TrustBackedAssetsPalletLocation, Balance>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    TrustBackedAssetsConvertedConcreteId,
    // Convert an XCM Location into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We only want to allow teleports of known assets. We use non-zero issuance as an indication
    // that this asset is known.
    LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, Assets>>,
    // The account to use for tracking teleports.
    CheckingAccount,
>;

// Using the latest `Location`, we don't need to worry about migrations.
pub type ForeignAssetsAssetId = Location;
/// `AssetId`/`Balance` converter for `ForeignAssets`.
pub type ForeignAssetsConvertedConcreteId = xcm_builder::MatchedConvertedConcreteId<
    Location,
    Balance,
    EverythingBut<(
        // Here we rely on fact that something like this works:
        // assert!(Location::new(1,
        // [Parachain(100)]).starts_with(&Location::parent()));
        // assert!([Parachain(100)].into().starts_with(&Here));
        StartsWith<assets_common::matching::LocalLocationPattern>,
    )>,
    Identity,
    TryConvertInto,
>;

/// Means for transacting foreign assets from different global consensus.
pub type ForeignFungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    ForeignAssets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ForeignAssetsConvertedConcreteId,
    // Convert an XCM Location into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't need to check teleports here.
    NoChecking,
    // The account to use for tracking teleports.
    CheckingAccount,
>;

/// `AssetId`/`Balance` converter for `PoolAssets`.
pub type PoolAssetsConvertedConcreteId =
    assets_common::PoolAssetsConvertedConcreteId<PoolAssetsPalletLocation, Balance>;

/// Means for transacting asset conversion pool assets on this chain.
pub type PoolFungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    PoolAssets,
    // Use this currency when it is a fungible asset matching the given location or name:
    PoolAssetsConvertedConcreteId,
    // Convert an XCM Location into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We only want to allow teleports of known assets. We use non-zero issuance as an indication
    // that this asset is known.
    LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, PoolAssets>>,
    // The account to use for tracking teleports.
    CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (
    FungibleTransactor,
    FungiblesTransactor,
    ForeignFungiblesTransactor,
    PoolFungiblesTransactor,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognized.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `RuntimeOrigin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
    pub XcmAssetFeesReceiver: Option<AccountId> = Authorship::author();
}

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
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Allow XCMs with some computed origins to pass through.
    WithComputedOrigin<
        (
            // If the message is one that immediately attempts to pay for execution, then
            // allow it.
            AllowTopLevelPaidExecutionFrom<Everything>,
            // Subscriptions for version tracking are OK.
            AllowSubscriptionsFrom<Everything>,
            // HRMP notifications from the relay chain are OK.
            AllowHrmpNotificationsFromRelayChain,
        ),
        UniversalLocation,
        ConstU32<8>,
    >,
)>;

/// Multiplier used for dedicated `TakeFirstAssetTrader` with `Assets` instance.
pub type AssetFeeAsExistentialDepositMultiplierFeeCharger = AssetFeeAsExistentialDepositMultiplier<
    Runtime,
    WeightToFee,
    pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, TrustBackedAssetsInstance>,
    TrustBackedAssetsInstance,
>;

/// Multiplier used for dedicated `TakeFirstAssetTrader` with `ForeignAssets` instance.
pub type ForeignAssetFeeAsExistentialDepositMultiplierFeeCharger =
    AssetFeeAsExistentialDepositMultiplier<
        Runtime,
        WeightToFee,
        pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, ForeignAssetsInstance>,
        ForeignAssetsInstance,
    >;

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = Equals<NativeCurrency>;

/// Asset converter for pool assets.
/// Used to convert assets in pools to the asset required for fee payment.
/// The pool must be between the first asset and the one required for fee payment.
/// This type allows paying fees with any asset in a pool with the asset required for fee payment.
pub type PoolAssetsExchanger = SingleAssetExchangeAdapter<
    crate::AssetConversion,
    crate::NativeAndNonPoolAssets,
    (
        TrustBackedAssetsAsLocation<
            TrustBackedAssetsPalletLocation,
            Balance,
            xcm::latest::Location,
        >,
        ForeignAssetsConvertedConcreteId,
    ),
    AccountId,
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type XcmEventEmitter = PolkadotXcm;
    // How to withdraw and deposit an asset.
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    // no teleport trust established with other chains
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = (
        UsingComponents<
            WeightToFee,
            NativeCurrency,
            AccountId,
            Balances,
            ResolveTo<StakingPot, Balances>,
        >,
        cumulus_primitives_utility::SwapFirstAssetTrader<
            NativeCurrency,
            AssetConversion,
            WeightToFee,
            super::NativeAndAssets,
            (
                TrustBackedAssetsAsLocation<
                    TrustBackedAssetsPalletLocation,
                    Balance,
                    xcm::latest::Location,
                >,
                ForeignAssetsConvertedConcreteId,
            ),
            ResolveAssetTo<StakingPot, super::NativeAndAssets>,
            AccountId,
        >, // add TakeFirstAssetTrader here if system parachain
    );
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type AssetLocker = ();
    type AssetExchanger = PoolAssetsExchanger;
    type FeeManager = XcmFeeManagerFromComponents<
        WaivedLocations,
        SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>,
    >;
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
    type Aliasers = TrustedAliasers;
    type TransactionalProcessor = FrameTransactionalProcessor;
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
    type XcmRecorder = PolkadotXcm;
}

/// Multiplier used for dedicated `TakeFirstAssetTrader` with `ForeignAssets` instance.
pub type ForeignAssetFeeAsExistentialDepositMultiplierFeeCharger =
    AssetFeeAsExistentialDepositMultiplier<
        Runtime,
        WeightToFee,
        pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, ForeignAssetsInstance>,
        ForeignAssetsInstance,
    >;

/// Converts a local signed origin into an XCM location. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub type PriceForParentDelivery =
    ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, ParachainSystem>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
)>;

parameter_types! {
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    pub const AuthorizeAliasHoldReason: RuntimeHoldReason = RuntimeHoldReason::PolkadotXcm(pallet_xcm::HoldReason::AuthorizeAlias);
}

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
    // ^ Override for AdvertisedXcmVersion default
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
    // xcm_executor::Config::Aliasers also uses pallet_xcm::AuthorizedAliasers.
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

/// Simple conversion of `u32` into an `AssetId` for use in benchmarking.
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<ForeignAssetsAssetId> for XcmBenchmarkHelper {
    fn create_asset_id_parameter(id: u32) -> ForeignAssetsAssetId {
        Location::new(1, [Parachain(id)])
    }
}
