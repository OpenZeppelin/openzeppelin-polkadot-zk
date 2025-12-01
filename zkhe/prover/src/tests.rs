use crate::*;
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT as G, ristretto::RistrettoPoint, scalar::Scalar,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Helper to generate random scalar using same method as prover (256-bit entropy)
fn random_scalar_test<R: RngCore>(rng: &mut R) -> Scalar {
    let mut bytes = [0u8; 64];
    rng.fill_bytes(&mut bytes);
    Scalar::from_bytes_mod_order_wide(&bytes)
}

#[test]
fn sender_receiver_round_trip_shapes() {
    let mut seed = [0u8; 32];
    seed[0] = 7;

    let sk_sender = Scalar::from(5u64);
    let pk_sender = sk_sender * G;
    let pk_receiver = Scalar::from(9u64) * G;

    let h = pedersen_h_generator();

    // Sender's initial (available) balance
    let from_old_v = 1234u64;
    let from_old_r = Scalar::from(42u64);
    let from_old_c = Scalar::from(from_old_v) * G + from_old_r * h;

    // Receiver: start with avail=0 and pending=ΔC so accepting will move pending→avail
    let avail_old_v = 0u64;
    let avail_old_r = Scalar::from(0u64);
    let avail_old_c = RistrettoPoint::identity();

    let dv = 111u64;

    // Sender phase: produce Δct, ΔC, bundle
    let s_in = SenderInput {
        asset_id: b"TEST_ASSET".to_vec(),
        network_id: [1u8; 32],
        sender_pk: pk_sender,
        receiver_pk: pk_receiver,
        from_old_c,
        from_old_opening: (from_old_v, from_old_r),
        to_old_c: RistrettoPoint::identity(), // receiver's pending not applied in phase 1
        delta_value: dv,
        rng_seed: seed,
        fee_c: None,
    };
    let s_out = prove_sender_transfer(&s_in).expect("sender prove");

    assert_eq!(s_out.delta_ct_bytes.len(), 64);
    assert!(s_out.sender_bundle_bytes.len() >= 32 + 192 + 2 + 1 + 2);

    // Deterministically recover rho to build receiver's pending opening
    // Must match the prover's random_scalar usage: k is first, rho is second
    let mut rng = ChaCha20Rng::from_seed(seed);
    let _k_ignored = random_scalar_test(&mut rng); // first draw = k (256-bit)
    let delta_rho = random_scalar_test(&mut rng); // second draw = rho (256-bit)

    // Build ΔC and pending_old (= ΔC) so accept will zero pending and raise available
    let delta_comm = CompressedRistretto(s_out.delta_comm_bytes)
        .decompress()
        .expect("ΔC decompress");
    // pending_old commitment equals ΔC; opening (dv, rho)
    let pending_old_v = dv;
    let pending_old_r = delta_rho;
    let pending_old_c = delta_comm;

    // Receiver acceptance with known witnesses (KISS mode)
    let r_in = ReceiverAcceptInput {
        asset_id: b"TEST_ASSET".to_vec(),
        network_id: [1u8; 32],
        receiver_pk: pk_receiver,
        // openings for both avail and pending, matching the verifier's semantics
        avail_old_c,
        avail_old_opening: (avail_old_v, avail_old_r),
        pending_old_c,
        pending_old_opening: (pending_old_v, pending_old_r),
        delta_comm,
        delta_value: dv,
        delta_rho,
    };

    let r_out = prove_receiver_accept(&r_in).expect("receiver accept");
    // env = 32 + 2 + len(rp_avail_new) + 2 + len(rp_pending_new)
    assert!(r_out.accept_envelope.len() > 32 + 2 + 2);
}
