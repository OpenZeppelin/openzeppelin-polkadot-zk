//! EVM-specific pallet configurations.

use super::*;
use crate::precompiles::FrontierPrecompiles;
use frame_support::parameter_types;
use pallet_evm::FeeCalculator;
use sp_core::U256;
use sp_runtime::traits::ConstU32;

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(75_000_000);
    pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<Runtime>::new();
    pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
    pub GasLimitPovSizeRatio: u64 = 4;
    pub GasLimitStorageGrowthRatio: u64 = 4;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        (U256::from(1_000_000_000u64), Weight::zero())
    }
}

impl pallet_evm_chain_id::Config for Runtime {}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    // CallOrigin restricted to root for direct Substrate->EVM calls.
    // This is intentional for this EVM rollup where all user EVM interactions
    // go through pallet_ethereum (Ethereum JSON-RPC), not direct EVM calls.
    // Direct EVM calls via Substrate are only used for governance/sudo operations.
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type PrecompilesType = FrontierPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = EVMChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = ();
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
    type Timestamp = Timestamp;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
    type WeightInfo = ();
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: pallet_ethereum::PostLogContent = pallet_ethereum::PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type StateRoot = pallet_ethereum::IntermediateStateRoot<super::Version>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
    pub DefaultElasticity: sp_runtime::Permill = sp_runtime::Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> sp_runtime::Permill {
        sp_runtime::Permill::zero()
    }
    fn ideal() -> sp_runtime::Permill {
        sp_runtime::Permill::from_parts(500_000)
    }
    fn upper() -> sp_runtime::Permill {
        sp_runtime::Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type Threshold = BaseFeeThreshold;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
}
