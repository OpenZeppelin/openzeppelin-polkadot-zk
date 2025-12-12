//! Mock runtime for testing the confidential assets EVM precompile.

use super::*;

use confidential_assets_primitives::{
    ConfidentialBackend, EncryptedAmount, PublicKeyBytes, Ramp, ZkVerifier,
};
use frame_support::{
    construct_runtime, derive_impl, parameter_types, traits::Everything, weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, FrameSystemAccountProvider};
use precompile_utils::{mock_account, precompile_set::*, testing::MockAccount};
use sp_core::{H256, U256};
use sp_runtime::{BuildStorage, Perbill, traits::BlakeTwo256};

pub type AccountId = MockAccount;
pub type AssetId = u128;
pub type Balance = u128;
pub type Block = frame_system::mocking::MockBlockU32<Runtime>;

// --- Mock verifier that always succeeds ---
#[derive(Default)]
pub struct AlwaysOkVerifier;

impl ZkVerifier for AlwaysOkVerifier {
    type Error = ();

    fn disclose(_asset: &[u8], _pk: &[u8], _cipher: &[u8]) -> Result<u64, ()> {
        Ok(123)
    }

    fn verify_transfer_sent(
        _asset: &[u8],
        _from_pk: &[u8],
        _to_pk: &[u8],
        _from_old_avail: &[u8],
        _to_old_pending: &[u8],
        _delta_ct: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ()> {
        Ok((vec![1u8; 32], vec![2u8; 32]))
    }

    fn verify_transfer_received(
        _asset: &[u8],
        _who_pk: &[u8],
        _avail_old: &[u8],
        _pending_old: &[u8],
        _commits: &[[u8; 32]],
        _envelope: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ()> {
        Ok((vec![3u8; 32], vec![0u8; 32]))
    }

    fn verify_mint(
        _asset: &[u8],
        _to_pk: &PublicKeyBytes,
        _to_old_pending: &[u8],
        _total_old: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, EncryptedAmount), ()> {
        Ok((vec![10u8; 32], vec![11u8; 32], [5u8; 64]))
    }

    fn verify_burn(
        _asset: &[u8],
        _from_pk: &PublicKeyBytes,
        _from_old_avail: &[u8],
        _total_old: &[u8],
        _amount_ct: &EncryptedAmount,
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, u64), ()> {
        Ok((vec![20u8; 32], vec![21u8; 32], 42))
    }
}

// --- Mock ramp that always succeeds ---
pub struct NoRamp;
impl Ramp<AccountId, AssetId, Balance> for NoRamp {
    type Error = ();

    fn transfer_from(
        _from: &AccountId,
        _to: &AccountId,
        _asset: AssetId,
        _amount: Balance,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn burn(_from: &AccountId, _asset: &AssetId, _amount: Balance) -> Result<(), Self::Error> {
        Ok(())
    }
    fn mint(_to: &AccountId, _asset: &AssetId, _amount: Balance) -> Result<(), Self::Error> {
        Ok(())
    }
}

construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Balances: pallet_balances,
        Timestamp: pallet_timestamp,
        Evm: pallet_evm,
        Zkhe: pallet_zkhe,
        ConfidentialAssets: pallet_confidential_assets,
    }
);

parameter_types! {
    pub const BlockHashCount: u32 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 1);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type BaseCallFilter = Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 0;
}

impl pallet_balances::Config for Runtime {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// Precompile addresses
pub const CONFIDENTIAL_ASSETS_PRECOMPILE: u64 = 2048;

pub type Precompiles<R> = PrecompileSetBuilder<
    R,
    (PrecompileAt<AddressU64<CONFIDENTIAL_ASSETS_PRECOMPILE>, ConfidentialAssetsPrecompile<R>>,),
>;

pub type PCall = ConfidentialAssetsPrecompileCall<Runtime>;

mock_account!(ConfidentialAssetsAddress, |_| MockAccount::from_u64(
    CONFIDENTIAL_ASSETS_PRECOMPILE
));

const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
const BLOCK_STORAGE_LIMIT: u64 = 40 * 1024;

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(u64::MAX);
    pub PrecompilesValue: Precompiles<Runtime> = Precompiles::new();
    pub const WeightPerGas: Weight = Weight::from_parts(1, 0);
    pub GasLimitPovSizeRatio: u64 = {
        let block_gas_limit = BlockGasLimit::get().min(u64::MAX.into()).low_u64();
        block_gas_limit.saturating_div(MAX_POV_SIZE)
    };
    pub GasLimitStorageGrowthRatio: u64 = {
        let block_gas_limit = BlockGasLimit::get().min(u64::MAX.into()).low_u64();
        block_gas_limit.saturating_div(BLOCK_STORAGE_LIMIT)
    };
}

impl pallet_evm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AccountId;
    type Currency = Balances;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = Precompiles<Runtime>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = BlockGasLimit;
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
    type AccountProvider = FrameSystemAccountProvider<Runtime>;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
}

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = AlwaysOkVerifier;
    type WeightInfo = ();
}

impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = NoRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}

pub(crate) struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> ExtBuilder {
        ExtBuilder { balances: vec![] }
    }
}

impl ExtBuilder {
    pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances,
            dev_accounts: None,
        }
        .assimilate_storage(&mut t)
        .expect("Pallet balances storage can be assimilated");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
        });
        ext
    }
}

pub fn precompiles() -> Precompiles<Runtime> {
    PrecompilesValue::get()
}

/// Helper to set a public key for an account
pub fn set_pk(who: AccountId) {
    <Zkhe as ConfidentialBackend<AccountId, AssetId, Balance>>::set_public_key(
        &who,
        &[7u8; 64].to_vec().try_into().expect("bounded vec"),
    )
    .unwrap();
}
