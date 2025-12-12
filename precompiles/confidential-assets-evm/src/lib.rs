//! EVM Precompile for Confidential Assets
//!
//! This precompile exposes the confidential assets pallet functionality
//! to Solidity contracts, following the moonbeam precompile pattern.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use confidential_assets_primitives::{EncryptedAmount, InputProof, PublicKeyBytes};
use fp_evm::PrecompileHandle;
use frame_support::{
    BoundedVec,
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::ConstU32,
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use precompile_utils::{
    evm::logs::{LogExt, log2, log3, log4},
    keccak256,
};
use sp_core::{H160, H256, U256};
use sp_runtime::traits::Dispatchable;

/// Size limits for bounded inputs (matching primitives)
pub const MAX_PROOF_SIZE: u32 = 8192;
pub const MAX_PUBKEY_SIZE: u32 = 64;
pub const ENCRYPTED_AMOUNT_SIZE: u32 = 64;

type GetMaxProofSize = ConstU32<MAX_PROOF_SIZE>;
type GetMaxPubKeySize = ConstU32<MAX_PUBKEY_SIZE>;
type GetEncryptedAmountSize = ConstU32<ENCRYPTED_AMOUNT_SIZE>;

/// Event selectors for EVM logs
/// event PublicKeySet(address indexed account, bytes pubkey)
pub const SELECTOR_LOG_PUBLIC_KEY_SET: [u8; 32] = keccak256!("PublicKeySet(address,bytes)");

/// event Deposit(uint128 indexed asset, address indexed account, uint256 amount)
pub const SELECTOR_LOG_DEPOSIT: [u8; 32] = keccak256!("Deposit(uint128,address,uint256)");

/// event Withdraw(uint128 indexed asset, address indexed account)
pub const SELECTOR_LOG_WITHDRAW: [u8; 32] = keccak256!("Withdraw(uint128,address)");

/// event ConfidentialTransfer(uint128 indexed asset, address indexed from, address indexed to)
pub const SELECTOR_LOG_CONFIDENTIAL_TRANSFER: [u8; 32] =
    keccak256!("ConfidentialTransfer(uint128,address,address)");

/// event ConfidentialClaim(uint128 indexed asset, address indexed account)
pub const SELECTOR_LOG_CONFIDENTIAL_CLAIM: [u8; 32] =
    keccak256!("ConfidentialClaim(uint128,address)");

/// Precompile exposing confidential assets functionality to EVM.
pub struct ConfidentialAssetsPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> ConfidentialAssetsPrecompile<Runtime>
where
    Runtime: pallet_confidential_assets::Config
        + pallet_evm::Config
        + pallet_zkhe::Config
        + frame_system::Config,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_confidential_assets::Call<Runtime>>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<<Runtime as frame_system::Config>::AccountId>>,
    <Runtime as pallet_evm::Config>::AddressMapping:
        AddressMapping<<Runtime as frame_system::Config>::AccountId>,
    <Runtime as pallet_confidential_assets::Config>::AssetId: TryFrom<u128> + Into<u128> + Copy,
    <Runtime as pallet_confidential_assets::Config>::Balance: TryFrom<U256> + Into<U256> + Copy,
{
    // ============ View Functions ============

    /// Returns the confidential balance commitment for an account.
    /// Solidity: function confidentialBalanceOf(uint128 asset, address who) view returns (bytes32)
    #[precompile::public("confidentialBalanceOf(uint128,address)")]
    #[precompile::view]
    fn confidential_balance_of(
        handle: &mut impl PrecompileHandle,
        asset: u128,
        who: Address,
    ) -> EvmResult<H256> {
        // Gas: DB read for balance
        handle.record_db_read::<Runtime>(64)?;

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;
        let who: <Runtime as frame_system::Config>::AccountId =
            <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(who.into());

        let commitment =
            pallet_confidential_assets::Pallet::<Runtime>::confidential_balance_of(asset_id, &who);

        Ok(H256::from_slice(&commitment))
    }

    /// Returns the confidential total supply commitment for an asset.
    /// Solidity: function confidentialTotalSupply(uint128 asset) view returns (bytes32)
    #[precompile::public("confidentialTotalSupply(uint128)")]
    #[precompile::view]
    fn confidential_total_supply(
        handle: &mut impl PrecompileHandle,
        asset: u128,
    ) -> EvmResult<H256> {
        // Gas: DB read for total supply
        handle.record_db_read::<Runtime>(32)?;

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let commitment =
            pallet_confidential_assets::Pallet::<Runtime>::confidential_total_supply(asset_id);

        Ok(H256::from_slice(&commitment))
    }

    /// Returns the asset name.
    /// Solidity: function name(uint128 asset) view returns (string)
    #[precompile::public("name(uint128)")]
    #[precompile::view]
    fn name(handle: &mut impl PrecompileHandle, asset: u128) -> EvmResult<UnboundedBytes> {
        handle.record_db_read::<Runtime>(64)?;

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let name = pallet_confidential_assets::Pallet::<Runtime>::asset_name(asset_id);
        Ok(name.into())
    }

    /// Returns the asset symbol.
    /// Solidity: function symbol(uint128 asset) view returns (string)
    #[precompile::public("symbol(uint128)")]
    #[precompile::view]
    fn symbol(handle: &mut impl PrecompileHandle, asset: u128) -> EvmResult<UnboundedBytes> {
        handle.record_db_read::<Runtime>(64)?;

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let symbol = pallet_confidential_assets::Pallet::<Runtime>::asset_symbol(asset_id);
        Ok(symbol.into())
    }

    /// Returns the asset decimals.
    /// Solidity: function decimals(uint128 asset) view returns (uint8)
    #[precompile::public("decimals(uint128)")]
    #[precompile::view]
    fn decimals(handle: &mut impl PrecompileHandle, asset: u128) -> EvmResult<u8> {
        handle.record_db_read::<Runtime>(8)?;

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        Ok(pallet_confidential_assets::Pallet::<Runtime>::asset_decimals(asset_id))
    }

    // ============ State-Changing Functions ============

    /// Sets the caller's public key for receiving confidential transfers.
    /// Solidity: function setPublicKey(bytes pubkey) external
    #[precompile::public("setPublicKey(bytes)")]
    fn set_public_key(
        handle: &mut impl PrecompileHandle,
        pubkey: BoundedBytes<GetMaxPubKeySize>,
    ) -> EvmResult {
        let caller = handle.context().caller;
        let origin = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(caller);
        let pubkey_vec: Vec<u8> = pubkey.into();
        let pubkey_bytes = pubkey_vec.clone();
        let pubkey_bounded: PublicKeyBytes =
            BoundedVec::try_from(pubkey_vec).map_err(|_| revert("pubkey too large"))?;

        // Dispatch the call
        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(origin).into(),
            pallet_confidential_assets::Call::<Runtime>::set_public_key {
                elgamal_pk: pubkey_bounded,
            },
            0,
        )?;

        // Emit PublicKeySet event
        // event PublicKeySet(address indexed account, bytes pubkey)
        log2(
            handle.context().address,
            SELECTOR_LOG_PUBLIC_KEY_SET,
            H256::from(caller),
            pubkey_bytes,
        )
        .record(handle)?;

        Ok(())
    }

    /// Deposits public assets into confidential balance (shield).
    /// Solidity: function deposit(uint128 asset, uint256 amount, bytes proof) external
    #[precompile::public("deposit(uint128,uint256,bytes)")]
    fn deposit(
        handle: &mut impl PrecompileHandle,
        asset: u128,
        amount: U256,
        proof: BoundedBytes<GetMaxProofSize>,
    ) -> EvmResult {
        let caller = handle.context().caller;
        let origin = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(caller);

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let balance: <Runtime as pallet_confidential_assets::Config>::Balance =
            amount.try_into().map_err(|_| revert("amount overflow"))?;

        let proof_vec: Vec<u8> = proof.into();
        let proof_bounded: InputProof =
            BoundedVec::try_from(proof_vec).map_err(|_| revert("proof too large"))?;

        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(origin).into(),
            pallet_confidential_assets::Call::<Runtime>::deposit {
                asset: asset_id,
                amount: balance,
                proof: proof_bounded,
            },
            0,
        )?;

        // Emit Deposit event
        // event Deposit(uint128 indexed asset, address indexed account, uint256 amount)
        // Use validated asset_id to ensure event matches state in case AssetId conversion is non-identity
        let asset_u128: u128 = asset_id.into();
        let mut asset_h256 = H256::zero();
        asset_h256.0[16..32].copy_from_slice(&asset_u128.to_be_bytes());
        let amount_bytes: [u8; 32] = amount.to_big_endian();
        log3(
            handle.context().address,
            SELECTOR_LOG_DEPOSIT,
            asset_h256,
            H256::from(caller),
            amount_bytes.to_vec(),
        )
        .record(handle)?;

        Ok(())
    }

    /// Withdraws confidential balance to public assets (unshield).
    /// Solidity: function withdraw(uint128 asset, bytes encryptedAmount, bytes proof) external
    #[precompile::public("withdraw(uint128,bytes,bytes)")]
    fn withdraw(
        handle: &mut impl PrecompileHandle,
        asset: u128,
        encrypted_amount: BoundedBytes<GetEncryptedAmountSize>,
        proof: BoundedBytes<GetMaxProofSize>,
    ) -> EvmResult {
        let caller = handle.context().caller;
        let origin = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(caller);

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let encrypted_vec: Vec<u8> = encrypted_amount.into();
        let encrypted_arr: EncryptedAmount = encrypted_vec
            .try_into()
            .map_err(|_| revert("encrypted amount must be 64 bytes"))?;

        let proof_vec: Vec<u8> = proof.into();
        let proof_bounded: InputProof =
            BoundedVec::try_from(proof_vec).map_err(|_| revert("proof too large"))?;

        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(origin).into(),
            pallet_confidential_assets::Call::<Runtime>::withdraw {
                asset: asset_id,
                encrypted_amount: encrypted_arr,
                proof: proof_bounded,
            },
            0,
        )?;

        // Emit Withdraw event
        // event Withdraw(uint128 indexed asset, address indexed account)
        // Use validated asset_id to ensure event matches state in case AssetId conversion is non-identity
        let asset_u128: u128 = asset_id.into();
        let mut asset_h256 = H256::zero();
        asset_h256.0[16..32].copy_from_slice(&asset_u128.to_be_bytes());
        log3(
            handle.context().address,
            SELECTOR_LOG_WITHDRAW,
            asset_h256,
            H256::from(caller),
            Vec::new(),
        )
        .record(handle)?;

        Ok(())
    }

    /// Performs a confidential transfer.
    /// Solidity: function confidentialTransfer(uint128 asset, address to, bytes encryptedAmount, bytes proof) external
    #[precompile::public("confidentialTransfer(uint128,address,bytes,bytes)")]
    fn confidential_transfer(
        handle: &mut impl PrecompileHandle,
        asset: u128,
        to: Address,
        encrypted_amount: BoundedBytes<GetEncryptedAmountSize>,
        proof: BoundedBytes<GetMaxProofSize>,
    ) -> EvmResult {
        let caller = handle.context().caller;
        let origin = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(caller);
        let to_h160: H160 = to.into();
        let to_account: <Runtime as frame_system::Config>::AccountId =
            <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(to_h160);

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let encrypted_vec: Vec<u8> = encrypted_amount.into();
        let encrypted_arr: EncryptedAmount = encrypted_vec
            .try_into()
            .map_err(|_| revert("encrypted amount must be 64 bytes"))?;

        let proof_vec: Vec<u8> = proof.into();
        let proof_bounded: InputProof =
            BoundedVec::try_from(proof_vec).map_err(|_| revert("proof too large"))?;

        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(origin).into(),
            pallet_confidential_assets::Call::<Runtime>::confidential_transfer {
                asset: asset_id,
                to: to_account,
                encrypted_amount: encrypted_arr,
                input_proof: proof_bounded,
            },
            0,
        )?;

        // Emit ConfidentialTransfer event
        // event ConfidentialTransfer(uint128 indexed asset, address indexed from, address indexed to)
        // Use validated asset_id to ensure event matches state in case AssetId conversion is non-identity
        let asset_u128: u128 = asset_id.into();
        let mut asset_h256 = H256::zero();
        asset_h256.0[16..32].copy_from_slice(&asset_u128.to_be_bytes());
        log4(
            handle.context().address,
            SELECTOR_LOG_CONFIDENTIAL_TRANSFER,
            asset_h256,
            H256::from(caller),
            H256::from(to_h160),
            Vec::new(),
        )
        .record(handle)?;

        Ok(())
    }

    /// Claims pending confidential deposits.
    /// Solidity: function confidentialClaim(uint128 asset, bytes proof) external
    #[precompile::public("confidentialClaim(uint128,bytes)")]
    fn confidential_claim(
        handle: &mut impl PrecompileHandle,
        asset: u128,
        proof: BoundedBytes<GetMaxProofSize>,
    ) -> EvmResult {
        let caller = handle.context().caller;
        let origin = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(caller);

        let asset_id = asset.try_into().map_err(|_| revert("invalid asset id"))?;

        let proof_vec: Vec<u8> = proof.into();
        let proof_bounded: InputProof =
            BoundedVec::try_from(proof_vec).map_err(|_| revert("proof too large"))?;

        RuntimeHelper::<Runtime>::try_dispatch(
            handle,
            Some(origin).into(),
            pallet_confidential_assets::Call::<Runtime>::confidential_claim {
                asset: asset_id,
                input_proof: proof_bounded,
            },
            0,
        )?;

        // Emit ConfidentialClaim event
        // event ConfidentialClaim(uint128 indexed asset, address indexed account)
        // Use validated asset_id to ensure event matches state in case AssetId conversion is non-identity
        let asset_u128: u128 = asset_id.into();
        let mut asset_h256 = H256::zero();
        asset_h256.0[16..32].copy_from_slice(&asset_u128.to_be_bytes());
        log3(
            handle.context().address,
            SELECTOR_LOG_CONFIDENTIAL_CLAIM,
            asset_h256,
            H256::from(caller),
            Vec::new(),
        )
        .record(handle)?;

        Ok(())
    }
}
