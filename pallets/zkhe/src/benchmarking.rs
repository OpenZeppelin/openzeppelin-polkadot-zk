//! Benchmarking setup for pallet-zkhe
//!
//! Uses pre-generated ZK proof vectors from zkhe-vectors crate.

use super::*;
use confidential_assets_primitives::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::vec::Vec;
use zkhe_vectors::*;

// ---- Helper functions ----

/// Setup sender public key from benchmark vectors
fn setup_sender_pk<T: Config>(who: &T::AccountId) {
    // SENDER_PK32 is [u8; 32], store as-is (verifier expects exactly 32 bytes)
    let pk_bytes: PublicKeyBytes = SENDER_PK32.to_vec().try_into().expect("32 bytes fits");
    PublicKey::<T>::insert(who, pk_bytes);
}

/// Setup receiver public key from benchmark vectors
fn setup_receiver_pk<T: Config>(who: &T::AccountId) {
    // RECEIVER_PK32 is [u8; 32], store as-is (verifier expects exactly 32 bytes)
    let pk_bytes: PublicKeyBytes = RECEIVER_PK32.to_vec().try_into().expect("32 bytes fits");
    PublicKey::<T>::insert(who, pk_bytes);
}

/// Setup sender's available balance commitment for transfer benchmarks
fn setup_sender_available_balance<T: Config>(asset: T::AssetId, who: &T::AccountId) {
    // Use TRANSFER_FROM_OLD_COMM_32 as the initial available balance
    AvailableBalanceCommit::<T>::insert(asset, who, TRANSFER_FROM_OLD_COMM_32);
}

/// Setup receiver's pending balance commitment (starts at identity/zero for fresh receiver)
fn setup_receiver_pending_balance<T: Config>(asset: T::AssetId, who: &T::AccountId) {
    // For a fresh receiver, pending starts at identity (all zeros = Ristretto identity point)
    // The vectors assume receiver pending starts empty, so we don't insert anything
    // or we can insert zero commitment if needed
    let _ = (asset, who); // silence unused warning
}

/// Setup pending deposits for accept_pending benchmarks
/// The pending deposit ciphertext must have C = TRANSFER_DELTA_COMM_32 as its first 32 bytes
fn setup_pending_deposit<T: Config>(asset: T::AssetId, who: &T::AccountId) {
    // The pallet extracts the first 32 bytes (C part) from the ciphertext to use as commitment
    // We need to construct a ciphertext where C = TRANSFER_DELTA_COMM_32
    // D part (second 32 bytes) can be from the actual delta ciphertext
    let mut fake_ct: [u8; 64] = [0u8; 64];
    fake_ct[0..32].copy_from_slice(&TRANSFER_DELTA_COMM_32);
    fake_ct[32..64].copy_from_slice(&TRANSFER_DELTA_CT_64[32..64]);

    // Insert the pending deposit UTXO at id=0
    PendingDeposits::<T>::insert((who.clone(), asset, 0u64), fake_ct);
    NextPendingDepositId::<T>::insert(who, asset, 1u64);

    // The pending balance commitment should be TRANSFER_DELTA_COMM_32
    // (this is the ΔC that the receiver will accept)
    // The accept envelope proves: avail_new = avail_old + ΔC, pending_new = pending_old - ΔC
    PendingBalanceCommit::<T>::insert(asset, who, TRANSFER_DELTA_COMM_32);
}

/// Build accept_input proof for accept_pending benchmark
/// Layout: u16 count || ids (u64 LE) * count || accept_envelope
fn build_accept_input(ids: &[u64], envelope: &[u8]) -> InputProof {
    let mut v = Vec::with_capacity(2 + ids.len() * 8 + envelope.len());
    let count = ids.len() as u16;
    v.extend_from_slice(&count.to_le_bytes());
    for id in ids {
        v.extend_from_slice(&id.to_le_bytes());
    }
    v.extend_from_slice(envelope);
    v.try_into().expect("bounded vec")
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn transfer() {
        let caller: T::AccountId = whitelisted_caller();
        let recipient: T::AccountId = account("recipient", 0, 0);
        let asset = T::AssetId::default();

        // Setup: both accounts need public keys, sender needs available balance
        setup_sender_pk::<T>(&caller);
        setup_receiver_pk::<T>(&recipient);
        setup_sender_available_balance::<T>(asset, &caller);
        setup_receiver_pending_balance::<T>(asset, &recipient);

        // Use real vectors
        let encrypted_amount: EncryptedAmount = TRANSFER_DELTA_CT_64;
        let proof: InputProof = Vec::from(TRANSFER_BUNDLE)
            .try_into()
            .expect("proof fits in BoundedVec<8192>");

        #[extrinsic_call]
        transfer(
            RawOrigin::Signed(caller.clone()),
            asset,
            recipient.clone(),
            encrypted_amount,
            proof,
        );

        // Verify state changed
        assert!(AvailableBalanceCommit::<T>::contains_key(asset, &caller));
        assert!(PendingBalanceCommit::<T>::contains_key(asset, &recipient));
    }

    #[benchmark]
    fn accept_pending() {
        let caller: T::AccountId = whitelisted_caller();
        let asset = T::AssetId::default();

        // Setup: receiver needs public key and pending deposits
        // For accept_pending, we use receiver vectors since they're accepting incoming funds
        setup_receiver_pk::<T>(&caller);
        setup_pending_deposit::<T>(asset, &caller);

        // Build accept envelope with deposit id=0 and the real accept envelope
        let accept_envelope = build_accept_input(&[0u64], ACCEPT_ENVELOPE);

        #[extrinsic_call]
        accept_pending(RawOrigin::Signed(caller.clone()), asset, accept_envelope);

        // Verify pending deposit was consumed
        assert!(!PendingDeposits::<T>::contains_key((
            caller.clone(),
            asset,
            0u64
        )));
    }

    // NOTE: accept_pending_and_transfer benchmark is not included because it requires
    // chained proofs where the accept result feeds into the transfer input.
    // The current vectors don't support this chaining.
    // Weight can be estimated as: accept_pending_weight + transfer_weight

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
