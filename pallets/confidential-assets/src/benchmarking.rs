//! Benchmarking for `pallet-confidential-assets`.

use crate::*;
use confidential_assets_primitives::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::vec::Vec;
use zkhe_vectors::*;

#[benchmarks(where T::AssetId: Default, T::Balance: From<u32>)]
mod benchmarks {
    use super::*;

    // ---- helpers ----
    fn asset<T: Config>() -> T::AssetId
    where
        T::AssetId: Default,
    {
        // Use default asset id for benchmarking
        Default::default()
    }

    #[inline]
    fn sender_pk() -> PublicKeyBytes {
        // SENDER_PK32 is [u8; 32], but PublicKeyBytes is BoundedVec<u8, 64>
        // Pad to 64 bytes
        let mut pk = [0u8; 64];
        pk[..32].copy_from_slice(&SENDER_PK32);
        pk.to_vec()
            .try_into()
            .expect("64 bytes fits in BoundedVec<64>")
    }

    #[inline]
    fn receiver_pk() -> PublicKeyBytes {
        let mut pk = [0u8; 64];
        pk[..32].copy_from_slice(&RECEIVER_PK32);
        pk.to_vec()
            .try_into()
            .expect("64 bytes fits in BoundedVec<64>")
    }

    // Ciphertexts - EncryptedAmount is [u8; 64]
    #[inline]
    fn ct_transfer() -> EncryptedAmount {
        TRANSFER_DELTA_CT_64
    }
    #[inline]
    fn ct_burn() -> EncryptedAmount {
        BURN_AMOUNT_CT_64
    }

    // Proofs / bundles - InputProof is BoundedVec<u8, 8192>
    #[inline]
    fn proof_mint() -> InputProof {
        Vec::from(MINT_PROOF)
            .try_into()
            .expect("proof fits in BoundedVec<8192>")
    }
    #[inline]
    fn proof_burn() -> InputProof {
        Vec::from(BURN_PROOF)
            .try_into()
            .expect("proof fits in BoundedVec<8192>")
    }
    #[inline]
    fn proof_transfer_bundle() -> InputProof {
        Vec::from(TRANSFER_BUNDLE)
            .try_into()
            .expect("proof fits in BoundedVec<8192>")
    }
    #[inline]
    fn proof_accept_envelope() -> InputProof {
        Vec::from(ACCEPT_ENVELOPE)
            .try_into()
            .expect("proof fits in BoundedVec<8192>")
    }

    // set_public_key(who, elgamal_pk)
    #[benchmark]
    fn set_public_key() {
        let who: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        set_public_key(RawOrigin::Signed(who), sender_pk());
    }

    // deposit(who, asset, amount, proof)  -> use prover-generated mint proof
    #[benchmark]
    fn deposit() {
        let who: T::AccountId = whitelisted_caller();

        // ensure key exists for the caller as most verifiers expect a PK on record
        Pallet::<T>::set_public_key(RawOrigin::Signed(who.clone()).into(), sender_pk()).unwrap();

        #[extrinsic_call]
        deposit(
            RawOrigin::Signed(who),
            asset::<T>(),
            1u32.into(),
            proof_mint(),
        );
    }

    // withdraw(who, asset, encrypted_amount, proof) -> burn path
    #[benchmark]
    fn withdraw() {
        let who: T::AccountId = whitelisted_caller();

        Pallet::<T>::set_public_key(RawOrigin::Signed(who.clone()).into(), sender_pk()).unwrap();

        #[extrinsic_call]
        withdraw(
            RawOrigin::Signed(who),
            asset::<T>(),
            ct_burn(),
            proof_burn(),
        );
    }

    // confidential_transfer(from, asset, to, encrypted_amount, input_proof)
    #[benchmark]
    fn confidential_transfer() {
        let from: T::AccountId = whitelisted_caller();
        let to: T::AccountId = account("to", 0, 0);

        // register both keys; many implementations require recipient PK known
        Pallet::<T>::set_public_key(RawOrigin::Signed(from.clone()).into(), sender_pk()).unwrap();
        Pallet::<T>::set_public_key(RawOrigin::Signed(to.clone()).into(), receiver_pk()).unwrap();

        #[extrinsic_call]
        confidential_transfer(
            RawOrigin::Signed(from),
            asset::<T>(),
            to,
            ct_transfer(),
            proof_transfer_bundle(),
        );
    }

    // disclose_amount(who, asset, encrypted_amount)
    // Use the same 64B transfer delta ciphertext for a deterministic vector.
    #[benchmark]
    fn disclose_amount() {
        let who: T::AccountId = whitelisted_caller();

        Pallet::<T>::set_public_key(RawOrigin::Signed(who.clone()).into(), sender_pk()).unwrap();

        #[extrinsic_call]
        disclose_amount(RawOrigin::Signed(who), asset::<T>(), ct_transfer());
    }

    // confidential_claim(who, asset, input_proof) -> accept-envelope vector
    #[benchmark]
    fn confidential_claim() {
        let who: T::AccountId = whitelisted_caller();

        Pallet::<T>::set_public_key(RawOrigin::Signed(who.clone()).into(), sender_pk()).unwrap();

        #[extrinsic_call]
        confidential_claim(
            RawOrigin::Signed(who),
            asset::<T>(),
            proof_accept_envelope(),
        );
    }

    // confidential_transfer_from(caller, asset, from, to, encrypted_amount, input_proof)
    // Use self-path to avoid ACL/operator setup, keep behavior identical.
    #[benchmark]
    fn confidential_transfer_from() {
        let from: T::AccountId = whitelisted_caller();
        let to: T::AccountId = account("to", 0, 0);

        Pallet::<T>::set_public_key(RawOrigin::Signed(from.clone()).into(), sender_pk()).unwrap();
        Pallet::<T>::set_public_key(RawOrigin::Signed(to.clone()).into(), receiver_pk()).unwrap();

        #[extrinsic_call]
        confidential_transfer_from(
            RawOrigin::Signed(from.clone()),
            asset::<T>(),
            from.clone(),
            to,
            ct_transfer(),
            proof_transfer_bundle(),
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
