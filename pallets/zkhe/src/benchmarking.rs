//! Benchmarking setup for pallet-zkhe

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

/// Helper to set up a public key for an account.
fn setup_public_key<T: Config>(who: &T::AccountId) {
    let pk: PublicKeyBytes = [7u8; 64].to_vec().try_into().expect("bounded vec");
    PublicKey::<T>::insert(who, pk);
}

/// Helper to set up available balance commitment for an account.
fn setup_available_balance<T: Config>(asset: T::AssetId, who: &T::AccountId) {
    AvailableBalanceCommit::<T>::insert(asset, who, [1u8; 32]);
}

/// Helper to set up pending deposits for accept_pending benchmarks.
fn setup_pending_deposits<T: Config>(asset: T::AssetId, who: &T::AccountId, count: u32) {
    for i in 0..count {
        PendingDeposits::<T>::insert((who.clone(), asset, i as u64), [5u8; 64]);
    }
    NextPendingDepositId::<T>::insert(who, asset, count as u64);
    // Also set pending balance commit
    PendingBalanceCommit::<T>::insert(asset, who, [2u8; 32]);
}

/// Build accept_input proof: u16 count || ids (u64 LE) * count || rest (opaque)
fn build_accept_input(ids: &[u64], rest: &[u8]) -> InputProof {
    let mut v = Vec::with_capacity(2 + ids.len() * 8 + rest.len());
    let count = ids.len() as u16;
    v.extend_from_slice(&count.to_le_bytes());
    for id in ids {
        v.extend_from_slice(&id.to_le_bytes());
    }
    v.extend_from_slice(rest);
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
        setup_public_key::<T>(&caller);
        setup_public_key::<T>(&recipient);
        setup_available_balance::<T>(asset, &caller);

        let encrypted_amount: EncryptedAmount = [0u8; 64];
        let proof: InputProof = vec![0u8; 32].try_into().expect("bounded vec");

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

        // Setup: public key and pending deposits
        setup_public_key::<T>(&caller);
        setup_pending_deposits::<T>(asset, &caller, 1);

        // Build accept envelope: 1 deposit id (0), plus opaque proof bytes
        let accept_envelope = build_accept_input(&[0u64], &[0u8; 32]);

        #[extrinsic_call]
        accept_pending(RawOrigin::Signed(caller.clone()), asset, accept_envelope);

        // Verify pending deposit was consumed
        assert!(!PendingDeposits::<T>::contains_key((
            caller.clone(),
            asset,
            0u64
        )));
    }

    #[benchmark]
    fn accept_pending_and_transfer() {
        let caller: T::AccountId = whitelisted_caller();
        let recipient: T::AccountId = account("recipient", 0, 0);
        let asset = T::AssetId::default();

        // Setup: both need public keys, caller needs pending deposits
        setup_public_key::<T>(&caller);
        setup_public_key::<T>(&recipient);
        setup_pending_deposits::<T>(asset, &caller, 1);
        // Also need available balance for the transfer part
        setup_available_balance::<T>(asset, &caller);

        let accept_envelope = build_accept_input(&[0u64], &[0u8; 32]);
        let transfer_proof: InputProof = vec![0u8; 32].try_into().expect("bounded vec");

        #[extrinsic_call]
        accept_pending_and_transfer(
            RawOrigin::Signed(caller.clone()),
            asset,
            recipient.clone(),
            accept_envelope,
            transfer_proof,
        );

        // Verify pending deposit was consumed and transfer occurred
        assert!(!PendingDeposits::<T>::contains_key((
            caller.clone(),
            asset,
            0u64
        )));
        assert!(PendingBalanceCommit::<T>::contains_key(asset, &recipient));
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
