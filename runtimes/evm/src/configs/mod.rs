//! Pallet configurations for the EVM runtime.

mod evm;

use super::*;

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
    derive_impl,
    traits::{AsEnsureOriginWithArg, TransformOrigin},
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::{BlockHashCount, xcm_sender::NoPriceForMessageDelivery};
use sp_runtime::traits::{AccountIdConversion, AccountIdLookup};

// Re-exports
pub use evm::*;

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    // Weight configuration for the EVM rollup.
    //
    // NOTE: Base weights are set to 0 because:
    // 1. This is an EVM-focused rollup where gas metering handles most fee calculation
    // 2. The TransactionByteFee (10 * MICRO_UNIT per byte) provides a floor for all transactions
    // 3. Native Substrate extrinsics are expected to be minimal (mostly EVM transactions)
    //
    // For production with significant native extrinsic usage, consider adding non-zero base weights
    // and proper DbWeight configuration.
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(Weight::from_parts(0, 0))
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = Weight::from_parts(0, 0);
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 42;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
    type AccountId = AccountId;
    type Nonce = Nonce;
    type Hash = Hash;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type DbWeight = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = ConstU32<16>;
    type Lookup = AccountIdLookup<AccountId, ()>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MICRO_UNIT;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = frame_support::weights::IdentityFee<Balance>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ();
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

/// Relay chain slot duration in milliseconds.
/// This should match the relay chain's slot duration (typically 6000ms for Polkadot).
pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

/// Block processing velocity - number of parachain blocks per relay chain slot.
/// A value of 1 means one parachain block per relay chain slot.
pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;

/// Unincluded segment capacity - maximum number of blocks that can be built
/// but not yet included in the relay chain.
pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;

impl cumulus_pallet_parachain_system::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
    type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
        Runtime,
        RELAY_CHAIN_SLOT_DURATION_MILLIS,
        BLOCK_PROCESSING_VELOCITY,
        UNINCLUDED_SEGMENT_CAPACITY,
    >;
    type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;
    type RelayParentOffset = sp_core::ConstU32<1>;
}

parameter_types! {
    pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
    type MaxStale = sp_core::ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = ();
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = sp_core::ConstU32<1_000>;
    type MaxActiveOutboundChannels = ConstU32<128>;
    type MaxPageSize = ConstU32<{ 1 << 16 }>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
    type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisablingStrategy = ();
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100_000>;
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
}

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type PotId = PotId;
    type MaxCandidates = ConstU32<100>;
    type MinEligibleCollators = ConstU32<4>;
    type MaxInvulnerables = ConstU32<20>;
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

// XCM Configuration
//
// IMPORTANT: XCM is intentionally disabled in this runtime.
//
// This is an EVM-focused confidential assets rollup where XCM cross-chain functionality
// is not required for the initial release. The XCM pallets are included for parachain
// infrastructure compatibility but are configured to reject all XCM messages.
//
// Configuration decisions:
// - XcmSender = (): No outbound XCM messages can be sent
// - AssetTransactor = (): No asset handling for XCM (prevents unintended asset movements)
// - Barrier = (): All inbound XCM messages are rejected (returns error to sender)
// - XcmOriginToTransactDispatchOrigin always returns Err, rejecting origin conversion
//
// If XCM functionality is needed in the future, this configuration must be updated with:
// - Proper barrier configuration (e.g., AllowTopLevelPaidExecutionFrom)
// - Asset transactor for handling reserve/teleport assets
// - Origin converters for proper dispatch
// - Real weight info instead of TestWeightInfo
parameter_types! {
    pub const RelayNetwork: Option<xcm::latest::NetworkId> = None;
    pub UniversalLocation: xcm::latest::InteriorLocation = xcm::latest::Junctions::Here;
    pub const UnitWeightCost: Weight = Weight::from_parts(1_000_000, 64 * 1024);
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    // Outbound XCM is disabled - no messages can be sent
    type XcmSender = ();
    // No asset handling - prevents unintended asset movements via XCM
    type AssetTransactor = ();
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    // Barrier = () means all inbound XCM messages are rejected
    type Barrier = ();
    type Weigher = xcm_builder::FixedWeightBounds<UnitWeightCost, RuntimeCall, ConstU32<100>>;
    type Trader = ();
    type ResponseHandler = ();
    type AssetTrap = ();
    type AssetClaims = ();
    type SubscriptionService = ();
    type PalletInstancesInfo = ();
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = ();
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = ();
    type Aliasers = ();
    type TransactionalProcessor = ();
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
    type XcmRecorder = ();
    type XcmEventEmitter = ();
}

pub struct XcmOriginToTransactDispatchOrigin;
impl xcm_executor::traits::ConvertOrigin<RuntimeOrigin> for XcmOriginToTransactDispatchOrigin {
    fn convert_origin(
        _origin: impl Into<xcm::latest::Location>,
        _kind: xcm::latest::OriginKind,
    ) -> Result<RuntimeOrigin, xcm::latest::Location> {
        Err(xcm::latest::Location::default())
    }
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = frame_system::EnsureNever<xcm::latest::Location>;
    type XcmRouter = ();
    type ExecuteXcmOrigin = frame_system::EnsureNever<xcm::latest::Location>;
    type XcmExecuteFilter = ();
    type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = ();
    type XcmReserveTransferFilter = ();
    type Weigher = xcm_builder::FixedWeightBounds<UnitWeightCost, RuntimeCall, ConstU32<100>>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = ();
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = ();
    type MaxLockers = ConstU32<8>;
    // NOTE: Using TestWeightInfo is acceptable here because XCM is disabled.
    // When XCM is enabled, replace with real benchmarked weights.
    type WeightInfo = pallet_xcm::TestWeightInfo;
    type AdminOrigin = EnsureRoot<AccountId>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AuthorizedAliasConsideration = ();
}

// Assets pallet for standard (non-confidential) assets
parameter_types! {
    pub const AssetDeposit: Balance = UNIT;
    pub const AssetAccountDeposit: Balance = UNIT;
    pub const ApprovalDeposit: Balance = 100 * MICRO_UNIT;
    pub const AssetsStringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = UNIT;
    pub const MetadataDepositPerByte: Balance = 10 * MICRO_UNIT;
    // Account that owns assets created via root. Derived from "py/asset" PalletId.
    pub AssetsPalletAccount: AccountId = PalletId(*b"py/asset").into_account_truncating();
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = u128;
    type AssetIdParameter = parity_scale_codec::Compact<u128>;
    type Currency = Balances;
    // Asset creation is restricted to root origin only. This prevents unauthorized users
    // from creating arbitrary asset IDs, which could collide with asset ID 0 (reserved as
    // the native asset sentinel by PublicRamp). Assets created via root will be owned by
    // the AssetsPalletAccount. Use `force_create` via sudo for asset creation.
    type CreateOrigin =
        AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, AssetsPalletAccount>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = AssetsStringLimit;
    type Holder = ();
    type Freezer = ();
    type Extra = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type CallbackHandle = ();
    type AssetAccountDeposit = AssetAccountDeposit;
    type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
}

// Confidential Assets configuration
impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = Balance;
    type Verifier = zkhe_verifier::ZkheVerifier<confidential_assets_primitives::ZeroNetworkId>;
    type WeightInfo = ();
}

// Minimal confidential assets config - using Ramp and Backend types
use confidential_assets_primitives::Ramp;
use frame_support::traits::{
    Currency, ExistenceRequirement, Get,
    tokens::fungibles::Mutate as MultiTransfer,
    tokens::{Fortitude, Precision, Preservation, WithdrawReasons},
};
use sp_runtime::DispatchError;

type BalancesPallet = pallet_balances::Pallet<Runtime>;
type AssetsPallet = pallet_assets::Pallet<Runtime>;

pub struct NativeAssetId;
impl frame_support::traits::Get<u128> for NativeAssetId {
    fn get() -> u128 {
        0
    }
}

#[inline]
fn is_native(asset: &u128) -> bool {
    *asset == NativeAssetId::get()
}

pub struct PublicRamp;
impl Ramp<AccountId, u128, Balance> for PublicRamp {
    type Error = DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: u128,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        if is_native(&asset) {
            <BalancesPallet as Currency<AccountId>>::transfer(
                from,
                to,
                amount,
                ExistenceRequirement::AllowDeath,
            )?;
        } else {
            <AssetsPallet as MultiTransfer<AccountId>>::transfer(
                asset,
                from,
                to,
                amount,
                Preservation::Expendable,
            )?;
        }
        Ok(())
    }

    fn mint(to: &AccountId, asset: &u128, amount: Balance) -> Result<(), Self::Error> {
        if is_native(asset) {
            let _imbalance = <BalancesPallet as Currency<AccountId>>::deposit_creating(to, amount);
        } else {
            <AssetsPallet as MultiTransfer<AccountId>>::mint_into(*asset, to, amount)?;
        }
        Ok(())
    }

    fn burn(from: &AccountId, asset: &u128, amount: Balance) -> Result<(), Self::Error> {
        if is_native(asset) {
            let _imbalance = <BalancesPallet as Currency<AccountId>>::withdraw(
                from,
                amount,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
        } else {
            <AssetsPallet as MultiTransfer<AccountId>>::burn_from(
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

impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u128;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = PublicRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}
