//! EVM Runtime with Confidential Assets
//!
//! This runtime provides EVM compatibility via Frontier pallets
//! and integrates confidential assets functionality.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod configs;
mod precompiles;

extern crate alloc;
use alloc::vec::Vec;

use frame_support::{
    PalletId,
    dispatch::DispatchClass,
    parameter_types,
    traits::{ConstBool, ConstU8, ConstU32, ConstU64, VariantCountOf},
    weights::{ConstantMultiplier, Weight, constants::WEIGHT_REF_TIME_PER_SECOND},
};
use frame_system::limits::{BlockLength, BlockWeights};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{
    MultiSignature, Perbill, generic, impl_opaque_keys,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use precompiles::FrontierPrecompiles;
pub use sp_runtime::{MultiAddress, OpaqueExtrinsic};

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type TxExtension = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("evm-confidential-runtime"),
    impl_name: alloc::borrow::Cow::Borrowed("evm-confidential-runtime"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

/// Block time constants
pub const MILLI_SECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;

pub const MINUTES: BlockNumber = 60_000 / (MILLI_SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// Currency units
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLI_UNIT: Balance = 1_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000;

pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
    cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

// Configure pallets in the configs module
pub use configs::*;

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Runtime;

    // System pallets
    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Timestamp = pallet_timestamp;
    #[runtime::pallet_index(2)]
    pub type ParachainSystem = cumulus_pallet_parachain_system;
    #[runtime::pallet_index(3)]
    pub type ParachainInfo = parachain_info;

    // Monetary pallets
    #[runtime::pallet_index(10)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(11)]
    pub type TransactionPayment = pallet_transaction_payment;
    #[runtime::pallet_index(12)]
    pub type Assets = pallet_assets;

    // Governance
    #[runtime::pallet_index(15)]
    pub type Sudo = pallet_sudo;

    // Consensus
    #[runtime::pallet_index(20)]
    pub type Authorship = pallet_authorship;
    #[runtime::pallet_index(21)]
    pub type CollatorSelection = pallet_collator_selection;
    #[runtime::pallet_index(22)]
    pub type Session = pallet_session;
    #[runtime::pallet_index(23)]
    pub type Aura = pallet_aura;
    #[runtime::pallet_index(24)]
    pub type AuraExt = cumulus_pallet_aura_ext;

    // XCM
    #[runtime::pallet_index(30)]
    pub type XcmpQueue = cumulus_pallet_xcmp_queue;
    #[runtime::pallet_index(31)]
    pub type PolkadotXcm = pallet_xcm;
    #[runtime::pallet_index(32)]
    pub type CumulusXcm = cumulus_pallet_xcm;
    #[runtime::pallet_index(33)]
    pub type MessageQueue = pallet_message_queue;

    // Confidential Assets
    #[runtime::pallet_index(40)]
    pub type Zkhe = pallet_zkhe;
    #[runtime::pallet_index(41)]
    pub type ConfidentialAssets = pallet_confidential_assets;

    // EVM
    #[runtime::pallet_index(50)]
    pub type Ethereum = pallet_ethereum;
    #[runtime::pallet_index(51)]
    pub type EVM = pallet_evm;
    #[runtime::pallet_index(52)]
    pub type EVMChainId = pallet_evm_chain_id;
    #[runtime::pallet_index(53)]
    pub type BaseFee = pallet_base_fee;
}

// Runtime API implementations
//
// NOTE: Runtime APIs are intentionally empty for this alpha release.
//
// This EVM-focused rollup primarily uses Ethereum JSON-RPC for client interactions,
// not native Substrate runtime APIs. The standard Substrate APIs (Core, BlockBuilder,
// TaggedTransactionQueue, etc.) are provided by derive_impl macros from the
// ParaChainDefaultConfig preset.
//
// Additional runtime APIs (e.g., for confidential assets queries) may be added in
// future releases as needed for tooling integration.
sp_api::decl_runtime_apis! {}

pub const RUNTIME_API_VERSIONS: sp_version::ApisVec = sp_version::create_apis_vec!([]);
