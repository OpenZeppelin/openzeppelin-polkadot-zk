//! Types and traits for confidential assets crates
use frame_support::{pallet_prelude::*, BoundedVec};
use sp_std::prelude::*;

/// ZK El Gamal Ciphertext
/// bytes 0..32 = Pederson commitment
/// bytes 33..64 = El Gamal decrypt handle
pub type EncryptedAmount = [u8; 64];
/// Commitment type (compressed Ristretto)
pub type Commitment = [u8; 32];

/// Proof/aux data blob used by the backend to validate encrypted transfers.
pub type MaxProofLen = ConstU32<8192>;
pub type InputProof = BoundedVec<u8, MaxProofLen>;

/// Optional data payload for `*_and_call` variants.
pub type MaxCallbackDataLen = ConstU32<4096>;
pub type CallbackData = BoundedVec<u8, MaxCallbackDataLen>;

/// Zether/Solana-style public key bytes (ElGamal or similar).
pub type MaxPubKeyLen = ConstU32<64>;
pub type PublicKeyBytes = BoundedVec<u8, MaxPubKeyLen>;

/// Backend that holds the **truth** for totals, balances, public keys, and executes transfers.
// TODO: consider extracting any functions require public balances generic type to simplify
pub trait ConfidentialBackend<AccountId, AssetId, Balance> {
    fn set_public_key(who: &AccountId, elgamal_pk: &PublicKeyBytes) -> Result<(), DispatchError>;

    // Read encrypted balances state
    fn total_supply(asset: AssetId) -> Commitment;
    fn balance_of(asset: AssetId, who: &AccountId) -> Commitment;

    fn disclose_amount(
        asset: AssetId,
        encrypted_amount: &EncryptedAmount,
        who: &AccountId,
    ) -> Result<Balance, DispatchError>;

    fn transfer_encrypted(
        asset: AssetId,
        from: &AccountId,
        to: &AccountId,
        encrypted_amount: EncryptedAmount,
        input_proof: InputProof,
    ) -> Result<EncryptedAmount, DispatchError>;

    fn claim_encrypted(
        asset: AssetId,
        from: &AccountId,
        input_proof: InputProof,
    ) -> Result<EncryptedAmount, DispatchError>;

    fn mint_encrypted(
        asset: AssetId,
        to: &AccountId,
        input_proof: InputProof,
    ) -> Result<EncryptedAmount, DispatchError>;

    fn burn_encrypted(
        asset: AssetId,
        from: &AccountId,
        amount: EncryptedAmount,
        input_proof: InputProof,
    ) -> Result<Balance, DispatchError>;
}

/// Adaptor signature functionality required for trustless cross chain atomic swaps
pub trait AdaptorSigBackend {
    /// The secret used to satisfy the hashlock (e.g., a Ristretto scalar encoding).
    type Secret: Parameter + MaxEncodedLen + TypeInfo + Copy + Default;

    /// The hashlock type stored/compared in the pallet (often `[u8; 32]`).
    type HashLock: Parameter + MaxEncodedLen + TypeInfo + Copy + Default;

    /// Compute the hashlock `H(secret)`.
    fn hash_secret(secret: &Self::Secret) -> Self::HashLock;

    /// Given (partial, final) Schnorr signatures on the same message,
    /// recover the secret: `s = (s_final - s_partial) mod n`.
    fn recover_secret_from_sigs(
        partial_sig: &[u8; 64],
        final_sig: &[u8; 64],
    ) -> Result<Self::Secret, DispatchError>;

    /// Verify adaptor signature correctness inside no_std.
    fn verify_adaptor_sig(
        msg: &[u8],
        pubkey: &[u8; 32],
        adaptor_partial: &[u8; 64],
    ) -> Result<(), DispatchError>;
}

/// Plaintext Escrow trust
pub trait EscrowTrust<AccountId, AssetId, Balance> {
    /// Move value from `who` into pallet escrow.
    fn escrow_lock(asset: AssetId, who: &AccountId, amount: Balance) -> Result<(), DispatchError>;

    /// Release escrowed value to `to` (on successful redeem).
    fn escrow_release(asset: AssetId, to: &AccountId, amount: Balance)
        -> Result<(), DispatchError>;

    /// Refund escrowed value to `to` (after timeout).
    fn escrow_refund(asset: AssetId, to: &AccountId, amount: Balance) -> Result<(), DispatchError>;
}

/// Confidential escrow
pub trait ConfidentialEscrow<AccountId, AssetId> {
    /// Move value from `who` into pallet escrow.
    fn escrow_lock(
        asset: AssetId,
        who: &AccountId,
        encrypted_amount: EncryptedAmount,
        proof: InputProof,
    ) -> Result<(), DispatchError>;

    /// Release escrowed value to `to` (on successful redeem).
    fn escrow_release(
        asset: AssetId,
        to: &AccountId,
        encrypted_amount: EncryptedAmount,
        proof: InputProof,
    ) -> Result<(), DispatchError>;

    /// Refund escrowed value to `to` (after timeout).
    fn escrow_refund(
        asset: AssetId,
        to: &AccountId,
        encrypted_amount: EncryptedAmount,
        proof: InputProof,
    ) -> Result<(), DispatchError>;
}

/// Trait so other pallets can open/cancel intents without extrinsics.
pub trait ConfidentialSwapIntents<AccountId, AssetId> {
    type SwapId;
    fn open_intent_cc(
        maker: &AccountId,
        counterparty: &AccountId,
        asset_a: AssetId,
        asset_b: AssetId,
        a_to_b_ct: EncryptedAmount,
        a_to_b_proof: InputProof,
        terms_hash: Option<[u8; 32]>,
    ) -> Result<Self::SwapId, DispatchError>;

    fn execute_intent_cc(
        who: &AccountId,
        id: Self::SwapId,
        b_to_a_ct: EncryptedAmount,
        b_to_a_proof: InputProof,
    ) -> Result<(Self::SwapId, EncryptedAmount), DispatchError>;

    fn cancel_intent_cc(maker: &AccountId, id: Self::SwapId) -> DispatchResult;
}

/// Off/On-ramp for the public side of an asset.
/// Semantics:
/// - `transfer` = move `amount` of `asset` from `who` to `to`.
/// - `burn` = move *public* funds from `who` into the pallet's custody
///             (so we can mint confidential).
/// - `mint` = move *public* funds from the pallet's custody out to `who`
///             (after we burn/debit confidential).
pub trait Ramp<AccountId, AssetId, Amount> {
    type Error;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Amount,
    ) -> Result<(), Self::Error>;
    fn burn(from: &AccountId, asset: &AssetId, amount: Amount) -> Result<(), Self::Error>;
    fn mint(to: &AccountId, asset: &AssetId, amount: Amount) -> Result<(), Self::Error>;
}

/// Metadata provider per asset (names, symbols, etc.).
pub trait AssetMetadataProvider<AssetId> {
    fn name(asset: AssetId) -> Vec<u8>;
    fn symbol(asset: AssetId) -> Vec<u8>;
    fn decimals(asset: AssetId) -> u8;
    fn contract_uri(asset: AssetId) -> Vec<u8>;
}

/// Abstract verifier boundary. Implement in the runtime.
///
// TODO:
// - verify_{mint, burn}_{to_send, received}
pub trait ZkVerifier {
    type Error;

    /// Sender phase: verify link/range (as implemented) and compute new commitments.
    /// Inputs:
    /// - `from_old_avail_commit`, `to_old_pending_commit`: 0 or 32 bytes
    /// - `delta_ct`: 64B ElGamal ciphertext (C||D)
    /// - `proof_bundle`: sender bundle bytes
    ///
    /// Returns:
    /// - (from_new_available_commit, to_new_pending_commit), both 32B
    fn verify_transfer_sent(
        asset: &[u8],
        from_pk: &[u8],
        to_pk: &[u8],
        from_old_avail_commit: &[u8], // empty => identity
        to_old_pending_commit: &[u8], // empty => identity
        delta_ct: &[u8],              // 64B
        proof_bundle: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error>;

    /// Receiver phase (Option A): accept selected UTXO deposits.
    /// Inputs:
    /// - `avail_old_commit`, `pending_old_commit`: 0 or 32 bytes
    /// - `pending_commits`: slice of 32B commitments for the consumed UTXOs (Σ must equal ΔC)
    /// - `accept_envelope`: delta_comm(32) || len1(2) || rp_avail_new || len2(2) || rp_pending_new
    ///
    /// Returns:
    /// - (avail_new_commit, pending_new_commit), both 32B
    fn verify_transfer_received(
        asset: &[u8],
        who_pk: &[u8],
        avail_old_commit: &[u8],      // empty => identity
        pending_old_commit: &[u8],    // empty => identity
        pending_commits: &[[u8; 32]], // UTXO C’s to sum
        accept_envelope: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error>;

    /// Mint: prove v ≥ 0, update pending(to) and total supply.
    /// The prover chooses a fresh ElGamal nonce for the minted ciphertext.
    /// Returns (to_new_pending_commit, total_new_commit, minted_ciphertext_64B).
    fn verify_mint(
        asset: &[u8],
        to_pk: &PublicKeyBytes,
        to_old_pending: &[u8],
        total_old: &[u8],
        proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, EncryptedAmount), ()>;

    /// Burn: prove ciphertext encrypts v under `from_pk`, v ≥ 0,
    /// and update available(from) and total supply downward by v.
    /// Returns (from_new_available_commit, total_new_commit, disclosed_amount_u64).
    fn verify_burn(
        asset: &[u8],
        from_pk: &PublicKeyBytes,
        from_old_available: &[u8],
        total_old: &[u8],
        amount_ciphertext: &EncryptedAmount,
        proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, u64), ()>;

    /// Optional disclosure
    fn disclose(asset: &[u8], who_pk: &[u8], cipher: &[u8]) -> Result<u64, Self::Error>;
}

// Operator

pub trait OperatorRegistry<AccountId, AssetId, BlockNumber> {
    /// Return true if `operator` is currently authorized to operate for (`holder`, `asset`) at `now`.
    fn is_operator(
        holder: &AccountId,
        asset: &AssetId,
        operator: &AccountId,
        now: BlockNumber,
    ) -> bool;
}

impl<AccountId, AssetId, BlockNumber> OperatorRegistry<AccountId, AssetId, BlockNumber> for () {
    fn is_operator(
        _holder: &AccountId,
        _asset: &AssetId,
        _operator: &AccountId,
        _now: BlockNumber,
    ) -> bool {
        false
    }
}

// ACL

#[derive(Clone, Copy, Encode, Decode, scale_info::TypeInfo)]
pub enum Op {
    Mint,
    Burn,
    Transfer,
    TransferFrom,
    Shield,   // public -> confidential
    Unshield, // confidential -> public
    AcceptPending,
    SetOperator,
}

#[derive(Encode, Decode, scale_info::TypeInfo, Default)]
pub struct AclCtx<Balance, AccountId, AssetId> {
    pub amount: Balance, // plaintext amount if relevant; 0 if not
    pub asset: AssetId,
    pub caller: AccountId,               // origin who signed the extrinsic
    pub owner: Option<AccountId>,        // on-behalf-of (transfer_from etc.)
    pub counterparty: Option<AccountId>, // receiver/sender if applicable
    pub opaque: sp_std::vec::Vec<u8>,    // future-proof (proof bytes, memo, etc.)
}

pub trait AclProvider<AccountId, AssetId, Balance> {
    /// Return Ok(()) to allow; Err(..) to block.
    fn authorize(op: Op, ctx: &AclCtx<Balance, AccountId, AssetId>) -> Result<(), DispatchError>;
}

impl<AccountId, AssetId, Balance> AclProvider<AccountId, AssetId, Balance> for () {
    #[inline]
    fn authorize(_: Op, _: &AclCtx<Balance, AccountId, AssetId>) -> Result<(), DispatchError> {
        Ok(())
    }
}

// Confidential Bridge types and traits

/// Local HRMP messenger abstraction used by confidential-bridge pallet
/// Minimal abstraction so runtimes can plug in pallet-xcm HRMP or any messenger.
/// Implement this in the runtime using pallet-xcm's `SendXcm` or a custom adapter.
pub trait HrmpMessenger {
    /// Send an opaque SCALE-encoded payload to `dest_para`.
    fn send(dest_para: u32, payload: Vec<u8>) -> Result<(), ()>;
}

/// Unique id for each outbound transfer.
pub type TransferId = u64;

/// A tiny packet we send over HRMP.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct BridgePacket<AccountId, AssetId> {
    /// Bridge transfer identifier (source side).
    pub transfer_id: TransferId,
    /// Destination account (assume 32 bytes for simplicity)
    pub dest_account: AccountId,
    /// Asset to move.
    pub asset: AssetId,
    /// 64-byte ElGamal ciphertext for the amount being bridged.
    pub encrypted_amount: EncryptedAmount,
    /// Opaque "accept/credit" envelope/proof for the destination backend.
    pub accept_envelope: InputProof,
}

/// Internal ledger of a pending outbound transfer.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct PendingTransfer<AccountId, AssetId, BlockNumber> {
    pub from: AccountId,
    pub dest_para: u32,
    pub dest_account: AccountId,
    pub asset: AssetId,
    pub encrypted_amount: EncryptedAmount,
    /// Block number after which the sender may cancel and refund.
    pub deadline: BlockNumber,
    /// True once a finalize path (success or refund) executed.
    pub completed: bool,
}

// Confidential cross-chain atomic swaps (see examples/confidential-xcm-bridge)

pub trait BridgeHtlc<AccountId, AssetId, Amount> {
    type HashLock;
    type Secret;

    /// Create + fund an HTLC. Returns an identifier you can mirror cross-chain.
    fn open_htlc(
        maker: &AccountId,
        taker: Option<AccountId>,
        asset: AssetId,
        amount: Amount,
        hashlock: Self::HashLock,
        // absolute expiry block; refunds become valid at `>= expiry`
        expiry: u32,
        // Optional partial/adaptor signature commitment (for adaptor flow).
        adaptor_partial: Option<Vec<u8>>,
    ) -> Result<u64, DispatchError>;

    /// Redeem by preimage (classic HTLC). Returns the secret (for relaying).
    fn redeem_with_secret(
        who: &AccountId,
        htlc_id: u64,
        secret: Self::Secret,
    ) -> Result<Self::Secret, DispatchError>;

    /// Redeem by final signature; pallet recovers the secret and returns it.
    fn redeem_with_adaptor_sig(
        who: &AccountId,
        htlc_id: u64,
        final_sig: Vec<u8>,
    ) -> Result<Self::Secret, DispatchError>;

    /// Refund after expiry (maker only).
    fn refund(who: &AccountId, htlc_id: u64) -> DispatchResult;
}
