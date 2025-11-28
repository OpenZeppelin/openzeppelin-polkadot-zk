//! Benchmarking for `pallet-confidential-assets`.
//!
//! Note: Most operations delegate to the Backend (pallet_zkhe), so weights should
//! be similar to the backend weights plus some overhead for the wrapper logic.

use crate::*;
use confidential_assets_primitives::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use zkhe_vectors::*;

#[benchmarks(where T::AssetId: Default, T::Balance: From<u32>)]
mod benchmarks {
    use super::*;

    #[inline]
    fn sender_pk() -> PublicKeyBytes {
        // Use exactly 32 bytes - verifier expects compressed Ristretto point
        SENDER_PK32
            .to_vec()
            .try_into()
            .expect("32 bytes fits in BoundedVec<64>")
    }

    // set_public_key(who, elgamal_pk)
    #[benchmark]
    fn set_public_key() {
        let who: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        set_public_key(RawOrigin::Signed(who), sender_pk());
    }

    // NOTE: deposit and withdraw benchmarks are omitted because they require:
    // 1. A working Ramp implementation with sufficient public token balance
    // 2. Proper mint/burn proof generation that matches the Ramp's internal accounting
    //
    // The main cryptographic weight comes from verify_mint/verify_burn which is
    // already captured in the backend pallet benchmarks.

    // NOTE: confidential_transfer, confidential_claim, confidential_transfer_from
    // and disclose_amount benchmarks are omitted because they all delegate to the
    // backend (pallet_zkhe) which has its own benchmarks.
    //
    // The weight for these operations should be:
    //   confidential_assets_weight = backend_weight + small_overhead
    //
    // where backend_weight is from pallet_zkhe benchmarks and small_overhead
    // accounts for the wrapper logic (event emission, etc.)

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
