//! PolkaVM Precompile for Confidential Assets Pallet
//!
//! This precompile exposes the confidential assets pallet functionality to
//! PolkaVM smart contracts via a Solidity-compatible ABI interface.
//!
//! The precompile is registered at address `0x0000000000000000000000000000000C010000`
//! (C01 = "Confidential 01")

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::num::NonZero;
use polkadot_sdk::pallet_revive::{
    self,
    precompiles::{
        AddressMatcher, Error, Ext, Precompile,
        alloy::{
            sol,
            sol_types::{Revert, SolValue},
        },
    },
};

#[cfg(test)]
mod tests;

/// The precompile address for confidential assets.
/// Using 0x0C01 prefix (C01 = Confidential 01)
/// This becomes address: 0x0000000000000000000000000000000C010000
pub const PRECOMPILE_ADDRESS: u16 = 0x0C01;

// Compile-time assertion that PRECOMPILE_ADDRESS is non-zero
const _: () = assert!(PRECOMPILE_ADDRESS != 0, "PRECOMPILE_ADDRESS must be non-zero");

/// Confidential Assets Precompile
///
/// Exposes confidential assets functionality via Solidity ABI:
/// - `confidentialBalance(uint128, bytes32)` - Get encrypted balance commitment
/// - `publicKey(bytes32)` - Get the public key for an account
/// - `totalSupply(uint128)` - Get total supply commitment for an asset
pub struct ConfidentialAssetsPrecompile<T>(core::marker::PhantomData<T>);

impl<T> Default for ConfidentialAssetsPrecompile<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

// Solidity function selectors (first 4 bytes of keccak256 hash of function signature)
// Currently only view functions are implemented. State-changing functions are planned for future versions.
pub mod selectors {
    /// confidentialBalance(uint128,address) -> bytes32
    pub const CONFIDENTIAL_BALANCE: [u8; 4] = [0x4c, 0x5b, 0x3e, 0x9d];
    /// publicKey(address) -> bytes32
    pub const PUBLIC_KEY: [u8; 4] = [0x68, 0x5e, 0x3b, 0x40];
    /// totalSupply(uint128) -> bytes32
    pub const TOTAL_SUPPLY: [u8; 4] = [0x18, 0x16, 0x0d, 0xdd];
}

// Define the Solidity interface using alloy's sol! macro
sol! {
    #[sol(all_derives)]
    interface IConfidentialAssets {
        function confidentialBalance(uint128 assetId, bytes32 account) external view returns (bytes32);
        function publicKey(bytes32 account) external view returns (bytes32);
        function totalSupply(uint128 assetId) external view returns (bytes32);
    }
}

/// Helper function to create a revert error
fn revert_error(msg: &str) -> Error {
    Error::Revert(Revert {
        reason: String::from(msg),
    })
}

/// Implementation of the Precompile trait for confidential assets
impl<T> Precompile for ConfidentialAssetsPrecompile<T>
where
    T: pallet_revive::Config + pallet_confidential_assets::Config + pallet_zkhe::Config,
    T::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
    <T as pallet_confidential_assets::Config>::AssetId: From<u128> + Into<u128>,
    <T as pallet_confidential_assets::Config>::Balance: From<u128> + Into<u128>,
{
    type T = T;

    /// The interface type using the generated Solidity interface
    type Interface = IConfidentialAssets::IConfidentialAssetsCalls;

    /// Fixed address matcher at 0x0C01
    /// Address format: 0x0000000000000000000000000000000C010000
    const MATCHER: AddressMatcher =
        AddressMatcher::Fixed(unsafe { NonZero::new_unchecked(PRECOMPILE_ADDRESS) });

    /// This precompile does not need contract info storage
    const HAS_CONTRACT_INFO: bool = false;

    fn call(
        _address: &[u8; 20],
        input: &Self::Interface,
        _env: &mut impl Ext<T = T>,
    ) -> Result<Vec<u8>, Error> {
        use IConfidentialAssets::IConfidentialAssetsCalls::*;

        match input {
            confidentialBalance(call) => {
                let asset_id: u128 = call.assetId;
                let account_bytes: [u8; 32] = call.account.into();
                let account: T::AccountId = account_bytes.into();

                let commitment = pallet_confidential_assets::Pallet::<T>::confidential_balance_of(
                    asset_id.into(),
                    &account,
                );

                // Return the commitment as bytes32
                let result: [u8; 32] = commitment
                    .try_into()
                    .map_err(|_| revert_error("Invalid commitment length"))?;
                Ok(result.abi_encode())
            }
            publicKey(call) => {
                let account_bytes: [u8; 32] = call.account.into();
                let account: T::AccountId = account_bytes.into();

                // Get public key from zkhe pallet using the storage getter
                let pk = pallet_zkhe::Pallet::<T>::public_key(&account);

                // Returns zero bytes32 if no public key is registered for the account.
                // Callers should check for zero return to distinguish "no key" from a valid key.
                let result: [u8; 32] = match pk {
                    Some(key) => key
                        .as_slice()
                        .try_into()
                        .map_err(|_| revert_error("Invalid key length"))?,
                    None => [0u8; 32],
                };
                Ok(result.abi_encode())
            }
            totalSupply(call) => {
                let asset_id: u128 = call.assetId;

                let commitment = pallet_confidential_assets::Pallet::<T>::confidential_total_supply(
                    asset_id.into(),
                );

                let result: [u8; 32] = commitment
                    .try_into()
                    .map_err(|_| revert_error("Invalid commitment length"))?;
                Ok(result.abi_encode())
            }
        }
    }
}

// ABI decoding helper functions for tests
#[cfg(any(test, feature = "std"))]
pub mod abi_helpers {
    use alloc::vec::Vec;

    /// Decode a u128 from a 32-byte ABI-encoded slot
    pub fn decode_u128(data: &[u8]) -> Result<u128, ()> {
        if data.len() < 32 {
            return Err(());
        }
        // u128 is right-aligned in 32 bytes
        let bytes: [u8; 16] = data[16..32].try_into().map_err(|_| ())?;
        Ok(u128::from_be_bytes(bytes))
    }

    /// Decode a u256 as usize (for offsets)
    pub fn decode_u256_as_usize(data: &[u8]) -> Result<usize, ()> {
        if data.len() < 32 {
            return Err(());
        }
        // We only care about the last 8 bytes for reasonable offsets
        let bytes: [u8; 8] = data[24..32].try_into().map_err(|_| ())?;
        Ok(u64::from_be_bytes(bytes) as usize)
    }

    /// Decode dynamic bytes from ABI-encoded data
    pub fn decode_dynamic_bytes(input: &[u8], offset: usize) -> Result<Vec<u8>, ()> {
        if offset + 32 > input.len() {
            return Err(());
        }

        // First 32 bytes at offset is the length
        let length = decode_u256_as_usize(&input[offset..offset + 32])?;

        if offset + 32 + length > input.len() {
            return Err(());
        }

        Ok(input[offset + 32..offset + 32 + length].to_vec())
    }
}
