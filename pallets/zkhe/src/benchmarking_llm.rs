//! Benchmarking

use frame_benchmarking::v2::*;
use frame_support::{
    traits::{EnsureOrigin, OriginFor},
    BoundedVec,
};
use frame_system::RawOrigin;

use super::*;
use crate::Pallet;
use confidential_assets_primitives::{EncryptedAmount, PublicKeyBytes};

// Mock values for benchmarking
const ASSET: u32 = 7;

fn setup_public_key<T: Config>(account: &T::AccountId) -> Result<(), DispatchError> {
    let pk_bytes: PublicKeyBytes = BoundedVec::try_from(vec![0u8; 64]).unwrap();
    PublicKey::<T>::insert(account, pk_bytes);
    Ok(())
}

fn setup_available_balance_commit<T: Config>(
    asset: T::AssetId,
    account: &T::AccountId,
) -> Result<(), DispatchError> {
    let commitment: Commitment = [0u8; 32];
    AvailableBalanceCommit::<T>::insert(asset, account, commitment);
    Ok(())
}

fn setup_pending_balance_commit<T: Config>(
    asset: T::AssetId,
    account: &T::AccountId,
) -> Result<(), DispatchError> {
    let commitment: Commitment = [0u8; 32];
    PendingBalanceCommit::<T>::insert(asset, account, commitment);
    Ok(())
}

fn setup_pending_deposit<T: Config>(
    asset: T::AssetId,
    account: &T::AccountId,
    id: u64,
) -> Result<(), DispatchError> {
    let encrypted_amount: EncryptedAmount = [0u8; 64];
    PendingDeposits::<T>::insert((account, asset, id), encrypted_amount);
    NextPendingDepositId::<T>::insert(account, asset, id + 1);
    Ok(())
}

fn setup_total_supply_commit<T: Config>(asset: T::AssetId) -> Result<(), DispatchError> {
    let commitment: Commitment = [0u8; 32];
    TotalSupplyCommit::<T>::insert(asset, commitment);
    Ok(())
}

fn create_bounded_vec<T: Config>(len: u32) -> BoundedVec<u8, ConstU32<8192>> {
    BoundedVec::try_from(vec![0u8; len as usize]).unwrap()
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn transfer() {
        let caller: T::AccountId = whitelisted_caller();
        let to: T::AccountId = account("to", 0, 0);
        let asset: T::AssetId = ASSET.into();
        let encrypted_amount: EncryptedAmount = [0u8; 64];
        let proof: InputProof = create_bounded_vec::<T>(1024);

        // Setup
        setup_public_key::<T>(&caller)?;
        setup_public_key::<T>(&to)?;
        setup_available_balance_commit::<T>(asset, &caller)?;
        setup_pending_balance_commit::<T>(asset, &to)?;

        #[extrinsic_call]
        transfer(RawOrigin::Signed(caller.clone()), asset, to.clone(), encrypted_amount, proof);

        // Verify
        assert!(AvailableBalanceCommit::<T>::contains_key(asset, caller));
        assert!(PendingBalanceCommit::<T>::contains_key(asset, to));
    }

    #[benchmark]
    fn accept_pending() {
        let caller: T::AccountId = whitelisted_caller();
        let asset: T::AssetId = ASSET.into();
        let accept_envelope: InputProof = create_bounded_vec::<T>(1024);

        // Setup
        setup_public_key::<T>(&caller)?;
        setup_available_balance_commit::<T>(asset, &caller)?;
        setup_pending_balance_commit::<T>(asset, &caller)?;
        setup_pending_deposit::<T>(asset, &caller, 0)?;

        #[extrinsic_call]
        accept_pending(RawOrigin::Signed(caller.clone()), asset, accept_envelope);

        // Verify
        assert!(AvailableBalanceCommit::<T>::contains_key(asset, caller));
    }

    #[benchmark]
    fn accept_pending_and_transfer() {
        let caller: T::AccountId = whitelisted_caller();
        let to: T::AccountId = account("to", 0, 0);
        let asset: T::AssetId = ASSET.into();
        let accept_envelope: InputProof = create_bounded_vec::<T>(1024);
        let transfer_proof: InputProof = create_bounded_vec::<T>(1024);

        // Setup
        setup_public_key::<T>(&caller)?;
        setup_public_key::<T>(&to)?;
        setup_available_balance_commit::<T>(asset, &caller)?;
        setup_pending_balance_commit::<T>(asset, &caller)?;
        setup_pending_deposit::<T>(asset, &caller, 0)?;

        #[extrinsic_call]
        accept_pending_and_transfer(
            RawOrigin::Signed(caller.clone()),
            asset,
            to.clone(),
            accept_envelope,
            transfer_proof,
        );

        // Verify
        assert!(AvailableBalanceCommit::<T>::contains_key(asset, caller));
        assert!(PendingBalanceCommit::<T>::contains_key(asset, to));
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}