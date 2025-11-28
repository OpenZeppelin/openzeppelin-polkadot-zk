//! Configuration for pallet-revive (PolkaVM smart contracts).
//!
//! This module configures pallet-revive and exposes the confidential assets
//! functionality to PolkaVM contracts via the `call_runtime` mechanism.
//!
//! Contracts can call confidential assets extrinsics using the `seal_call_runtime`
//! syscall with SCALE-encoded calls to the ConfidentialAssets pallet.

use super::*;
use frame_support::traits::{ConstBool, ConstU32, ConstU64, Contains};

parameter_types! {
    /// Price of a byte of storage per one block interval. Should be greater than 0.
    pub const DepositPerItem: Balance = UNIT / 1000; // 0.001 UNIT per item
    /// Price of a byte of storage per one block interval. Should be greater than 0.
    pub const DepositPerByte: Balance = UNIT / 100_000; // 0.00001 UNIT per byte
    /// Percent of code size that is reserved for the lockup deposit.
    pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

/// Filter for calls that contracts are allowed to make via `seal_call_runtime`.
///
/// This allows contracts to call confidential assets pallet extrinsics.
pub struct ContractCallFilter;

impl Contains<RuntimeCall> for ContractCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(
            call,
            // Allow all confidential assets calls
            RuntimeCall::ConfidentialAssets(_)
        )
    }
}

impl pallet_revive::Config for Runtime {
    type Time = Timestamp;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    /// Allow contracts to call confidential assets pallet.
    type CallFilter = ContractCallFilter;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_revive::weights::SubstrateWeight<Self>;
    /// No chain extension - contracts use call_runtime instead.
    type ChainExtension = ();
    type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
    /// Maximum memory size for the runtime (128 MiB).
    type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
    /// Maximum memory size for PVF (512 MiB).
    type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
    /// Disable unsafe/unstable interface (use false for production).
    type UnsafeUnstableInterface = ConstBool<false>;
    /// Who can upload code.
    type UploadOrigin = EnsureSigned<Self::AccountId>;
    /// Who can instantiate contracts.
    type InstantiateOrigin = EnsureSigned<Self::AccountId>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    /// XCM interface for cross-chain contract calls.
    type Xcm = pallet_xcm::Pallet<Self>;
    /// Ethereum-compatible chain ID. Using a placeholder value.
    type ChainId = ConstU64<420_420_420>;
    /// Ratio of native token to ETH (10^(18-12) = 10^6 for 12 decimal native).
    type NativeToEthRatio = ConstU32<1_000_000>;
    /// Ethereum gas encoder (none for now).
    type EthGasEncoder = ();
    /// Block author finder for miner coinbase.
    type FindAuthor = <Runtime as pallet_authorship::Config>::FindAuthor;
}

/// Required conversion implementation for pallet-revive.
impl TryFrom<RuntimeCall> for pallet_revive::Call<Runtime> {
    type Error = ();

    fn try_from(value: RuntimeCall) -> core::result::Result<Self, Self::Error> {
        match value {
            RuntimeCall::Revive(call) => Ok(call),
            _ => Err(()),
        }
    }
}
