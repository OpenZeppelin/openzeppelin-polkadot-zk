//! Precompile set for the EVM runtime.
//!
//! This module defines the precompiles available to EVM contracts.

use core::marker::PhantomData;

use confidential_assets_evm_precompile::ConfidentialAssetsPrecompile;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{
    AddressMapping, IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult,
    PrecompileSet,
};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use sp_core::H160;
use sp_runtime::traits::Dispatchable;

/// Address for the confidential assets precompile (0x800 = 2048)
pub const CONFIDENTIAL_ASSETS_PRECOMPILE: u64 = 2048;

/// The precompile set for the EVM runtime.
#[derive(Default)]
pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Returns the list of addresses used by precompiles.
    pub fn used_addresses() -> [H160; 8] {
        [
            hash(1),                              // ECRecover
            hash(2),                              // Sha256
            hash(3),                              // Ripemd160
            hash(4),                              // Identity
            hash(5),                              // Modexp
            hash(1024),                           // Sha3FIPS256
            hash(1025),                           // ECRecoverPublicKey
            hash(CONFIDENTIAL_ASSETS_PRECOMPILE), // Confidential Assets
        ]
    }
}

impl<R> PrecompileSet for FrontierPrecompiles<R>
where
    R: pallet_evm::Config
        + pallet_confidential_assets::Config
        + pallet_zkhe::Config
        + frame_system::Config,
    <R as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <R as frame_system::Config>::RuntimeCall: From<pallet_confidential_assets::Call<R>>,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<<R as frame_system::Config>::AccountId>>,
    <R as pallet_evm::Config>::AddressMapping:
        AddressMapping<<R as frame_system::Config>::AccountId>,
    <R as pallet_confidential_assets::Config>::AssetId: TryFrom<u128> + Into<u128> + Copy,
    <R as pallet_confidential_assets::Config>::Balance:
        TryFrom<sp_core::U256> + Into<sp_core::U256> + Copy,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Ethereum precompiles
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            // Non-standard precompiles
            a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
            // Confidential Assets precompile
            a if a == hash(CONFIDENTIAL_ASSETS_PRECOMPILE) => Some(
                <ConfidentialAssetsPrecompile<R> as Precompile>::execute(handle),
            ),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
