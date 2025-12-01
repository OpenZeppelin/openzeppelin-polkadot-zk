//! Unit tests for the no_std ZK ElGamal verifier using pre-generated vectors.
//! Covered:
//!   1) Happy path: sender + receiver proofs verify and new commitments match vectors
//!   2) Rejection: tampered sender bundle is rejected
//!   3) Range proof only: parse sender bundle, reconstruct transcript context, and verify range proof

use confidential_assets_primitives::ZkVerifier as ZkVerifierTrait;
use confidential_assets_primitives::{EncryptedAmount, NetworkIdProvider, PublicKeyBytes};
use core::convert::TryFrom;
use curve25519_dalek::{
    ristretto::RistrettoPoint,
    traits::{Identity, IsIdentity},
};
use zkhe_primitives::RangeProofVerifier;
// Import the verifier marker struct from the crate root and its range verifier.
use crate::{BulletproofRangeVerifier, ZkheVerifier};
// Pre-generated deterministic vectors
use zkhe_vectors::*;

/// Test network ID provider - returns all zeros to match the network_id used
/// when generating test vectors.
pub struct TestNetworkId;
impl NetworkIdProvider for TestNetworkId {
    fn network_id() -> [u8; 32] {
        [0u8; 32]
    }
}

/// Test verifier with zero network ID (matches vector generation)
type TestVerifier = ZkheVerifier<TestNetworkId>;

// ---------- Bundle parsing (mirrors the on-chain verifier’s parsing logic) ----------
#[allow(unused)]
#[derive(Debug)]
struct ParsedSenderBundle<'a> {
    delta_comm: RistrettoPoint, // ΔC
    link_raw_192: [u8; 192],    // A1||A2||A3||z_k||z_v||z_r
    range_from_new: &'a [u8],   // bytes
    range_to_new: &'a [u8],     // bytes (often empty)
}

fn parse_sender_bundle(bytes: &[u8]) -> Result<ParsedSenderBundle<'_>, ()> {
    use zkhe_primitives::point_from_bytes;

    if bytes.len() < 32 + 192 + 2 + 2 {
        return Err(());
    }
    // ΔC
    let mut c = [0u8; 32];
    c.copy_from_slice(&bytes[0..32]);
    let delta_comm = point_from_bytes(&c).map_err(|_| ())?;

    // Link proof (192 bytes)
    let mut link_raw_192 = [0u8; 192];
    link_raw_192.copy_from_slice(&bytes[32..224]);

    // Lengths and slices
    let mut off = 224;
    let len1 = u16::from_le_bytes([bytes[off], bytes[off + 1]]) as usize;
    off += 2;
    if bytes.len() < off + len1 + 2 {
        return Err(());
    }
    let range1 = &bytes[off..off + len1];
    off += len1;

    let len2 = u16::from_le_bytes([bytes[off], bytes[off + 1]]) as usize;
    off += 2;
    if bytes.len() < off + len2 {
        return Err(());
    }
    let range2 = &bytes[off..off + len2];

    Ok(ParsedSenderBundle {
        delta_comm,
        link_raw_192,
        range_from_new: range1,
        range_to_new: range2,
    })
}

// Rebuild the exact transcript context the prover used for the sender’s range proof.
fn sender_range_context_from_bundle(
    asset_id_raw: &[u8],
    sender_pk: &RistrettoPoint,
    receiver_pk: &RistrettoPoint,
    delta_ct_bytes: &[u8; 64],
    link_raw_192: &[u8; 192],
) -> [u8; 32] {
    use curve25519_dalek::ristretto::CompressedRistretto;
    use zkhe_primitives::{
        append_point, challenge_scalar as fs_chal, labels, new_transcript, Ciphertext,
        PublicContext, SDK_VERSION,
    };

    let ct = Ciphertext::from_bytes(delta_ct_bytes).expect("valid delta ct");

    // Canonical public context (verifier fixes network_id = [0;32])
    let mut asset_id = [0u8; 32];
    if asset_id_raw.len() >= 32 {
        asset_id.copy_from_slice(&asset_id_raw[..32]);
    } else {
        asset_id[..asset_id_raw.len()].copy_from_slice(asset_id_raw);
    }
    let ctx = PublicContext {
        network_id: [0u8; 32],
        sdk_version: SDK_VERSION,
        asset_id,
        sender_pk: *sender_pk,
        receiver_pk: *receiver_pk,
        auditor_pk: None,
        fee_commitment: RistrettoPoint::identity(),
        ciphertext_out: ct,
        ciphertext_in: None,
    };
    let mut t = new_transcript(&ctx);

    // Unpack A1||A2||A3 from the 192-byte link proof prefix.
    let a1 = CompressedRistretto(link_raw_192[0..32].try_into().unwrap())
        .decompress()
        .expect("A1");
    let a2 = CompressedRistretto(link_raw_192[32..64].try_into().unwrap())
        .decompress()
        .expect("A2");
    let a3 = CompressedRistretto(link_raw_192[64..96].try_into().unwrap())
        .decompress()
        .expect("A3");

    append_point(&mut t, b"a1", &a1);
    append_point(&mut t, b"a2", &a2);
    append_point(&mut t, b"a3", &a3);

    // Challenge for the Σ-link (advances transcript)
    let _c = fs_chal(&mut t, labels::CHAL_EQ);

    // Squeeze the same 32 bytes the prover used to bind the range proof
    let mut ctx_bytes = [0u8; 32];
    let mut t_clone = t.clone();
    t_clone.challenge_bytes(b"ctx", &mut ctx_bytes);
    ctx_bytes
}

// ----------------------------- TESTS -----------------------------

#[test]
fn verify_sender_and_receiver_happy_path() {
    use curve25519_dalek::ristretto::CompressedRistretto;

    // Inputs from vectors
    let asset_id = ASSET_ID_BYTES;
    let _pk_sender_pt = CompressedRistretto(SENDER_PK32).decompress().expect("pk_s");
    let pk_receiver_pt = CompressedRistretto(RECEIVER_PK32)
        .decompress()
        .expect("pk_r");

    // Old commitments
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = RistrettoPoint::identity();

    // Sender phase verify
    let (from_new_bytes_v, to_new_pending_bytes_v) =
        <TestVerifier as ZkVerifierTrait>::verify_transfer_sent(
            asset_id,
            &SENDER_PK32,
            &RECEIVER_PK32,
            &from_old_c.compress().to_bytes(),
            &to_old_c.compress().to_bytes(),
            &TRANSFER_DELTA_CT_64,
            TRANSFER_BUNDLE,
        )
        .expect("sender-side verification failed");

    assert_eq!(from_new_bytes_v.as_slice(), &TRANSFER_FROM_NEW_COMM_32);
    assert_eq!(to_new_pending_bytes_v.as_slice(), &TRANSFER_TO_NEW_COMM_32);

    // Receiver phase verify
    let avail_old_c = RistrettoPoint::identity();
    let pending_old_c = CompressedRistretto(TRANSFER_DELTA_COMM_32)
        .decompress()
        .expect("ΔC");

    let pending_commits: Vec<[u8; 32]> = vec![pending_old_c.compress().to_bytes()];

    let (avail_new_bytes_v, pending_new_bytes_v) =
        <TestVerifier as ZkVerifierTrait>::verify_transfer_received(
            asset_id,
            &pk_receiver_pt.compress().to_bytes(),
            &avail_old_c.compress().to_bytes(),
            &pending_old_c.compress().to_bytes(),
            &pending_commits,
            ACCEPT_ENVELOPE,
        )
        .expect("receiver acceptance failed");

    assert_eq!(avail_new_bytes_v.as_slice(), &ACCEPT_AVAIL_NEW_COMM_32);
    assert_eq!(pending_new_bytes_v.as_slice(), &ACCEPT_PENDING_NEW_COMM_32);
}

#[test]
fn rejects_tampered_sender_bundle() {
    use curve25519_dalek::ristretto::CompressedRistretto;

    let asset_id = ASSET_ID_BYTES;
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = RistrettoPoint::identity();

    // Copy and tamper inside the link proof region.
    let mut bundle = TRANSFER_BUNDLE.to_vec();
    if bundle.len() >= 33 {
        bundle[32 + 10] ^= 0x01;
    }

    let err = <TestVerifier as ZkVerifierTrait>::verify_transfer_sent(
        asset_id,
        &SENDER_PK32,
        &RECEIVER_PK32,
        &from_old_c.compress().to_bytes(),
        &to_old_c.compress().to_bytes(),
        &TRANSFER_DELTA_CT_64,
        &bundle,
    );

    assert!(err.is_err(), "tampered sender bundle must be rejected");
}

#[test]
fn range_proof_from_sender_bundle_verifies() {
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek_ng::ristretto::CompressedRistretto as CNg;

    // Parse bundle and rebuild context from vectors
    let parsed = parse_sender_bundle(TRANSFER_BUNDLE).expect("parse bundle");

    let pk_sender_pt = CompressedRistretto(SENDER_PK32).decompress().expect("pk_s");
    let pk_receiver_pt = CompressedRistretto(RECEIVER_PK32)
        .decompress()
        .expect("pk_r");

    let ctx_bytes = sender_range_context_from_bundle(
        ASSET_ID_BYTES,
        &pk_sender_pt,
        &pk_receiver_pt,
        &TRANSFER_DELTA_CT_64,
        &parsed.link_raw_192,
    );

    // Verify the sender’s range proof against the bound commitment
    let mut commit32 = [0u8; 32];
    commit32.copy_from_slice(&TRANSFER_FROM_NEW_COMM_32);

    // Round-trip sanity via dalek-ng
    let v = CNg(commit32);
    let v_pt = v.decompress().expect("commit dec");
    let rt = v_pt.compress().to_bytes();
    assert_eq!(rt, commit32, "commit roundtrip mismatch");

    // Range proof verify
    match BulletproofRangeVerifier::verify_range_proof(
        b"range_from_new",
        &ctx_bytes,
        &commit32,
        parsed.range_from_new,
    ) {
        Ok(()) => {}
        Err(()) => {
            panic!("range proof should verify");
        }
    }
}

#[test]
fn identity_commitment_is_zero_point() {
    let zero = RistrettoPoint::default();
    assert!(zero.is_identity());
}

#[test]
fn mint_round_trip() {
    // to_pk from vectors. Old commits are identity, so pass empty slices.
    let to_pk_bv = PublicKeyBytes::try_from(RECEIVER_PK32.to_vec()).expect("pk bv");

    let (to_new_bytes, total_new_bytes, minted_ct_bytes) =
        <TestVerifier as ZkVerifierTrait>::verify_mint(
            ASSET_ID_BYTES,
            &to_pk_bv,
            &[], // to_old_pending = identity
            &[], // total_old = identity
            MINT_PROOF,
        )
        .expect("mint verify");

    assert_eq!(minted_ct_bytes, MINTED_CT_64);
    assert_eq!(to_new_bytes.as_slice(), &MINT_TO_NEW_COMM_32);
    assert_eq!(total_new_bytes.as_slice(), &MINT_TOTAL_NEW_COMM_32);
}

#[test]
fn burn_round_trip() {
    use curve25519_dalek::ristretto::CompressedRistretto;

    let from_pk_bv = PublicKeyBytes::try_from(SENDER_PK32.to_vec()).expect("pk bv");
    let from_old_c = CompressedRistretto(BURN_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let total_old_c = CompressedRistretto(BURN_TOTAL_OLD_COMM_32)
        .decompress()
        .expect("total_old");

    let amount_ct_bv = EncryptedAmount::try_from(BURN_AMOUNT_CT_64.to_vec()).expect("ct bv");

    let (from_new_bytes, total_new_bytes, disclosed) =
        <TestVerifier as ZkVerifierTrait>::verify_burn(
            ASSET_ID_BYTES,
            &from_pk_bv,
            &from_old_c.compress().to_bytes(), // from_old_available
            &total_old_c.compress().to_bytes(), // total_old
            &amount_ct_bv,                     // ciphertext of dv under from_pk
            BURN_PROOF,
        )
        .expect("burn verify");

    assert_eq!(disclosed, 120u64);
    assert_eq!(from_new_bytes.as_slice(), &BURN_FROM_NEW_COMM_32);
    assert_eq!(total_new_bytes.as_slice(), &BURN_TOTAL_NEW_COMM_32);
}
