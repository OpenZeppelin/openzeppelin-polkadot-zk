//! Confidential Assets interface for PolkaVM contracts.
//!
//! This crate provides type definitions and helpers for PolkaVM contracts
//! to interact with the confidential assets pallet via `seal_call_runtime`.
//!
//! # Usage from Contracts
//!
//! PolkaVM contracts can call confidential assets functionality using the
//! `seal_call_runtime` syscall with SCALE-encoded pallet calls:
//!
//! ```ignore
//! // Example: Transfer confidential assets
//! let call = pallet_confidential_assets::Call::confidential_transfer {
//!     asset,
//!     to,
//!     encrypted_amount,
//!     input_proof,
//! };
//! let encoded = call.encode();
//! seal_call_runtime(&encoded);
//! ```
//!
//! # Available Extrinsics
//!
//! The following confidential assets extrinsics are available to contracts:
//!
//! - `confidential_transfer` - Transfer encrypted amounts between accounts
//! - `deposit` - Shield public assets to confidential
//! - `withdraw` - Unshield confidential assets to public
//! - `set_public_key` - Set the encryption public key for an account
//! - `confidential_claim` - Claim pending deposits

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use confidential_assets_primitives::{
    Commitment, ConfidentialBackend, EncryptedAmount, InputProof, PublicKeyBytes, Ramp,
};
pub use parity_scale_codec::{Decode, Encode};

/// Helper types for encoding confidential assets calls from contracts.
pub mod calls {
    use super::*;

    /// Input for confidential_transfer call.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct ConfidentialTransferInput<AssetId, AccountId> {
        pub asset: AssetId,
        pub to: AccountId,
        pub encrypted_amount: EncryptedAmount,
        pub input_proof: InputProof,
    }

    /// Input for deposit call.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct DepositInput<AssetId, Balance> {
        pub asset: AssetId,
        pub amount: Balance,
        pub proof: InputProof,
    }

    /// Input for withdraw call.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct WithdrawInput<AssetId> {
        pub asset: AssetId,
        pub encrypted_amount: EncryptedAmount,
        pub proof: InputProof,
    }

    /// Input for set_public_key call.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct SetPublicKeyInput {
        pub elgamal_pk: PublicKeyBytes,
    }

    /// Input for confidential_claim call.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct ClaimInput<AssetId> {
        pub asset: AssetId,
        pub input_proof: InputProof,
    }
}

/// View function helpers for reading confidential assets state.
pub mod views {
    use super::*;

    /// Query confidential balance of an account.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct BalanceOfQuery<AssetId, AccountId> {
        pub asset: AssetId,
        pub who: AccountId,
    }

    /// Query confidential total supply.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct TotalSupplyQuery<AssetId> {
        pub asset: AssetId,
    }

    /// Query asset metadata.
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
    pub struct AssetQuery<AssetId> {
        pub asset: AssetId,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_are_encodable() {
        let input = calls::SetPublicKeyInput {
            elgamal_pk: [0u8; 32],
        };
        let encoded = input.encode();
        assert!(!encoded.is_empty());
    }
}
