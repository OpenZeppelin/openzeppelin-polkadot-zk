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
    // Must match the prover's random_scalar usage (256-bit entropy)
    let mut chacha = ChaCha20Rng::from_seed(seed);
    let mut bytes = [0u8; 64];
    chacha.fill_bytes(&mut bytes); // skip first scalar (k)
    chacha.fill_bytes(&mut bytes); // second scalar = rho
    let delta_rho = Scalar::from_bytes_mod_order_wide(&bytes);

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

    // ===================== EDGE CASE: LARGE VALUE MINT =====================
    // Test near-maximum value (2^63 - 1 fits in i64; use smaller for range proof)
    let mut seed_large = [0u8; 32];
    seed_large[0] = 0xBB;
    let large_value = 1_000_000_000u64; // 1 billion

    let large_mint = MintInput {
        asset_id: asset_id.clone(),
        network_id,
        to_pk: pk_receiver,
        to_pending_old_c: RistrettoPoint::identity(),
        to_pending_old_opening: (0, Scalar::from(0u64)),
        total_old_c: RistrettoPoint::identity(),
        total_old_opening: (0, Scalar::from(0u64)),
        mint_value: large_value,
        rng_seed: seed_large,
    };
    let large_mout = prove_mint(&large_mint).expect("large mint prover");

    // ===================== EDGE CASE: FULL BALANCE BURN =====================
    // Burn entire balance (from_new should be zero commitment)
    let mut seed_full = [0u8; 32];
    seed_full[2] = 0xFF;

    let full_burn_v = 1000u64;
    let full_burn_r = Scalar::from(777u64);
    let full_burn_c = Scalar::from(full_burn_v) * G + full_burn_r * h;

    let full_burn = BurnInput {
        asset_id: asset_id.clone(),
        network_id,
        from_pk: pk_sender,
        from_avail_old_c: full_burn_c,
        from_avail_old_opening: (full_burn_v, full_burn_r),
        total_old_c: full_burn_c,
        total_old_opening: (full_burn_v, full_burn_r),
        burn_value: full_burn_v, // burn entire balance
        rng_seed: seed_full,
    };
    let full_bout = prove_burn(&full_burn).expect("full burn prover");

    // ===================== MALFORMED PROOF VECTORS (for negative testing) =====================
    // These are intentionally malformed bytes that should cause verification to fail

    // Truncated bundle (too short)
    let truncated_bundle: Vec<u8> = sender_bundle[..100].to_vec();

    // Tampered bundle (flip bits in the proof)
    let mut tampered_bundle = sender_bundle.clone();
    tampered_bundle[50] ^= 0xFF; // flip bits at position 50
    tampered_bundle[100] ^= 0xFF; // flip bits at position 100

    // Invalid point (not on curve)
    let invalid_point: [u8; 32] = [0xFF; 32]; // all 1s is unlikely to be a valid point

    // ===================== EXPORT =====================
    format!(
        r#"// Auto-generated by bench_vector.rs.
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

// ===== EDGE CASE VECTORS =====

// ----- Large value mint (1 billion) -----
pub const LARGE_MINT_VALUE: u64 = {large_mint_value};
pub const LARGE_MINT_PROOF: &[u8] = &{large_mint_proof:?};
pub const LARGE_MINT_CT_64: [u8;64] = {large_mint_ct:?};
pub const LARGE_MINT_TO_NEW_COMM_32: [u8;32] = {large_mint_to_new:?};
pub const LARGE_MINT_TOTAL_NEW_COMM_32: [u8;32] = {large_mint_total_new:?};

// ----- Full balance burn (burn entire balance to zero) -----
pub const FULL_BURN_VALUE: u64 = {full_burn_value};
pub const FULL_BURN_PROOF: &[u8] = &{full_burn_proof:?};
pub const FULL_BURN_CT_64: [u8;64] = {full_burn_ct:?};
pub const FULL_BURN_FROM_OLD_COMM_32: [u8;32] = {full_burn_from_old:?};
pub const FULL_BURN_FROM_NEW_COMM_32: [u8;32] = {full_burn_from_new:?};
pub const FULL_BURN_TOTAL_NEW_COMM_32: [u8;32] = {full_burn_total_new:?};

// ===== NEGATIVE TEST VECTORS (should fail verification) =====

// ----- Truncated bundle (too short to parse) -----
pub const MALFORMED_TRUNCATED_BUNDLE: &[u8] = &{truncated:?};

// ----- Tampered bundle (valid length but corrupted proof) -----
pub const MALFORMED_TAMPERED_BUNDLE: &[u8] = &{tampered:?};

// ----- Invalid point (not on curve) -----
pub const MALFORMED_INVALID_POINT: [u8;32] = {invalid_pt:?};
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
        // edge case: large mint
        large_mint_value = large_value,
        large_mint_proof = large_mout.proof_bytes,
        large_mint_ct = large_mout.minted_ct_bytes,
        large_mint_to_new = large_mout.to_pending_new_c,
        large_mint_total_new = large_mout.total_new_c,
        // edge case: full burn
        full_burn_value = full_burn_v,
        full_burn_proof = full_bout.proof_bytes,
        full_burn_ct = full_bout.amount_ct_bytes,
        full_burn_from_old = to_bytes32(&full_burn_c),
        full_burn_from_new = full_bout.from_avail_new_c,
        full_burn_total_new = full_bout.total_new_c,
        // negative test vectors
        truncated = truncated_bundle,
        tampered = tampered_bundle,
        invalid_pt = invalid_point,
    )
}
