use crate::{
    BurnInput, MintInput, ReceiverAcceptInput, SenderInput, prove_burn, prove_mint,
    prove_receiver_accept, prove_sender_transfer,
};
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT as G, scalar::Scalar, traits::Identity,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sha2::Sha512;

fn pedersen_h() -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(b"Zether/PedersenH")
}
fn to_bytes32(pt: &RistrettoPoint) -> [u8; 32] {
    pt.compress().to_bytes()
}

/// Generate deterministic vectors for transfer, accept, mint, and burn.
/// Returned string is written to `zkhe_vectors/src/proofs.rs` or similar.
pub fn some_valid_proofs() -> String {
    // ---- common params ----
    // Use SCALE-encoded u128 = 0 to match runtime's T::AssetId::default()
    // u128 encodes as 16 bytes little-endian
    let asset_id = vec![0u8; 16];
    let network_id = [0u8; 32];

    // ---- keys ----
    let sk_sender = Scalar::from(5u64);
    let pk_sender = sk_sender * G;
    let sk_receiver = Scalar::from(9u64);
    let pk_receiver = sk_receiver * G;

    // ---- commitments/openings ----
    let h = pedersen_h();

    // Sender starts with 1_234 available
    let from_old_v = 1_234u64;
    let from_old_r = Scalar::from(42u64);
    let from_old_c = Scalar::from(from_old_v) * G + from_old_r * h;

    // Receiver avail=0, pending=ΔC will be seeded for claim
    let avail_old_v = 0u64;
    let avail_old_r = Scalar::from(0u64);
    let avail_old_c = RistrettoPoint::identity();

    let dv = 111u64;

    // ===================== SENDER TRANSFER =====================
    let mut seed = [0u8; 32];
    seed[0] = 7;

    let s_in = SenderInput {
        asset_id: asset_id.clone(),
        network_id,
        sender_pk: pk_sender,
        receiver_pk: pk_receiver,
        from_old_c,
        from_old_opening: (from_old_v, from_old_r),
        to_old_c: RistrettoPoint::identity(),
        delta_value: dv,
        rng_seed: seed,
        fee_c: None,
    };
    let s_out = prove_sender_transfer(&s_in).expect("sender prover");

    // Re-derive rho used by accept from the same seed
    let mut chacha = ChaCha20Rng::from_seed(seed);
    let _k_ignore = chacha.next_u64();
    let delta_rho = Scalar::from(chacha.next_u64());

    // Bytes from sender output
    let delta_ct_bytes = s_out.delta_ct_bytes; // 64
    let delta_comm_bytes = s_out.delta_comm_bytes; // 32
    let sender_bundle = s_out.sender_bundle_bytes; // var
    let from_new_bytes = s_out.from_new_c; // 32
    let to_new_bytes = s_out.to_new_c; // 32

    // For completeness compute from_new via algebra too (not exported)
    // let delta_c = CompressedRistretto(delta_comm_bytes).decompress().unwrap();
    // let from_new_check = (from_old_c - delta_c).compress().to_bytes();

    // ===================== RECEIVER ACCEPT =====================
    let delta_comm = {
        use curve25519_dalek::ristretto::CompressedRistretto;
        CompressedRistretto(delta_comm_bytes).decompress().unwrap()
    };

    let r_in = ReceiverAcceptInput {
        asset_id: asset_id.clone(),
        network_id,
        receiver_pk: pk_receiver,
        avail_old_c,
        avail_old_opening: (avail_old_v, avail_old_r),
        pending_old_c: delta_comm,
        pending_old_opening: (dv, delta_rho),
        delta_comm,
        delta_value: dv,
        delta_rho,
    };
    let r_out = prove_receiver_accept(&r_in).expect("receiver accept");

    // ===================== MINT =====================
    let mut seed_m = [0u8; 32];
    seed_m[0] = 0xA5;

    let min = MintInput {
        asset_id: asset_id.clone(),
        network_id,
        to_pk: pk_receiver,
        to_pending_old_c: RistrettoPoint::identity(),
        to_pending_old_opening: (0, Scalar::from(0u64)),
        total_old_c: RistrettoPoint::identity(),
        total_old_opening: (0, Scalar::from(0u64)),
        mint_value: 77,
        rng_seed: seed_m,
    };
    let mout = prove_mint(&min).expect("mint prover");

    // ===================== BURN =====================
    let mut seed_b = [0u8; 32];
    seed_b[1] = 0x5C;

    let from_old_v_b = 500u64;
    let from_old_r_b = Scalar::from(333u64);
    let from_old_c_b = Scalar::from(from_old_v_b) * G + from_old_r_b * h;

    let total_old_v_b = 500u64;
    let total_old_r_b = Scalar::from(111u64);
    let total_old_c_b = Scalar::from(total_old_v_b) * G + total_old_r_b * h;

    let bin = BurnInput {
        asset_id: asset_id.clone(),
        network_id,
        from_pk: pk_sender,
        from_avail_old_c: from_old_c_b,
        from_avail_old_opening: (from_old_v_b, from_old_r_b),
        total_old_c: total_old_c_b,
        total_old_opening: (total_old_v_b, total_old_r_b),
        burn_value: 120,
        rng_seed: seed_b,
    };
    let bout = prove_burn(&bin).expect("burn prover");

    // ===================== EXPORT =====================
    format!(
        r#"
// Auto-generated by bench_vector.rs.
// Deterministic vectors for verifier tests, runtime benches, and XCM tests.

// Asset ID is SCALE-encoded u128 = 0 (16 bytes of zeros)
pub const ASSET_ID_BYTES: [u8; 16] = [0u8; 16];
pub const SENDER_PK32:   [u8;32] = {sender_pk:?};
pub const RECEIVER_PK32: [u8;32] = {receiver_pk:?};

// ----- Transfer (sender) -----
pub const TRANSFER_FROM_OLD_COMM_32: [u8;32] = {transfer_from_old:?};
pub const TRANSFER_DELTA_CT_64:      [u8;64] = {delta_ct:?};
pub const TRANSFER_DELTA_COMM_32:    [u8;32] = {delta_comm:?};
pub const TRANSFER_BUNDLE:           &[u8]    = &{bundle:?};
pub const TRANSFER_FROM_NEW_COMM_32: [u8;32] = {transfer_from_new:?};
pub const TRANSFER_TO_NEW_COMM_32:   [u8;32] = {transfer_to_new:?};

// ----- Receiver accept -----
pub const ACCEPT_ENVELOPE:              &[u8]    = &{accept_env:?};
pub const ACCEPT_AVAIL_NEW_COMM_32:     [u8;32] = {accept_avail_new:?};
pub const ACCEPT_PENDING_NEW_COMM_32:   [u8;32] = {accept_pending_new:?};

// ----- Mint -----
pub const MINT_PROOF:            &[u8]    = &{mint_proof:?};
pub const MINTED_CT_64:          [u8;64]  = {minted_ct:?};
pub const MINT_TO_NEW_COMM_32:   [u8;32]  = {mint_to_new:?};
pub const MINT_TOTAL_NEW_COMM_32:[u8;32]  = {mint_total_new:?};

// ----- Burn -----
pub const BURN_AMOUNT_CT_64:     [u8;64]  = {burn_ct:?};
pub const BURN_PROOF:            &[u8]    = &{burn_proof:?};
pub const BURN_FROM_OLD_COMM_32: [u8;32]  = {burn_from_old:?};
pub const BURN_TOTAL_OLD_COMM_32:[u8;32]  = {burn_total_old:?};
pub const BURN_FROM_NEW_COMM_32: [u8;32]  = {burn_from_new:?};
pub const BURN_TOTAL_NEW_COMM_32:[u8;32]  = {burn_total_new:?};
"#,
        // keys
        sender_pk = to_bytes32(&pk_sender),
        receiver_pk = to_bytes32(&pk_receiver),
        // transfer
        transfer_from_old = to_bytes32(&from_old_c),
        delta_ct = delta_ct_bytes,
        delta_comm = delta_comm_bytes,
        bundle = sender_bundle,
        transfer_from_new = from_new_bytes,
        transfer_to_new = to_new_bytes,
        // accept
        accept_env = r_out.accept_envelope,
        accept_avail_new = r_out.avail_new_c,
        accept_pending_new = r_out.pending_new_c,
        // mint
        mint_proof = mout.proof_bytes,
        minted_ct = mout.minted_ct_bytes,
        mint_to_new = mout.to_pending_new_c,
        mint_total_new = mout.total_new_c,
        // burn
        burn_ct = bout.amount_ct_bytes,
        burn_proof = bout.proof_bytes,
        burn_from_old = to_bytes32(&from_old_c_b),
        burn_total_old = to_bytes32(&total_old_c_b),
        burn_from_new = bout.from_avail_new_c,
        burn_total_new = bout.total_new_c,
    )
}
