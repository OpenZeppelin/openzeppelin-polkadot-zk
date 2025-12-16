//! Tests using pre-generated vectors from zkhe-vectors
//!
//! These tests verify that the XCM confidential transfer flow works with
//! deterministic, pre-computed proofs. This enables reproducible testing
//! without needing to regenerate proofs each test run.

use confidential_assets_primitives::NetworkIdProvider;
use zkhe_vectors::*;

/// Test network ID provider - returns zero to match vector generation.
pub struct TestNetworkId;
impl NetworkIdProvider for TestNetworkId {
    fn network_id() -> [u8; 32] {
        [0u8; 32]
    }
}

/// Type alias for verifier with test network ID
type TestVerifier = zkhe_verifier::ZkheVerifier<TestNetworkId>;

/// Verifier test: verify transfer_sent with pre-generated vectors
#[test]
fn verify_transfer_sent_with_vectors() {
    use confidential_assets_primitives::ZkVerifier;
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek::traits::Identity;

    // Use vectors directly
    let asset_id = ASSET_ID_BYTES;
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();

    // Verify sender proof
    let result = <TestVerifier as ZkVerifier>::verify_transfer_sent(
        asset_id,
        &SENDER_PK32,
        &RECEIVER_PK32,
        &from_old_c.compress().to_bytes(),
        &to_old_c.compress().to_bytes(),
        &TRANSFER_DELTA_CT_64,
        TRANSFER_BUNDLE,
    );

    assert!(result.is_ok(), "verify_transfer_sent should succeed");

    let (from_new_bytes, to_new_pending_bytes) = result.unwrap();

    // Verify commitments match expected vectors
    assert_eq!(
        from_new_bytes.as_slice(),
        &TRANSFER_FROM_NEW_COMM_32,
        "from_new commitment mismatch"
    );
    assert_eq!(
        to_new_pending_bytes.as_slice(),
        &TRANSFER_TO_NEW_COMM_32,
        "to_new_pending commitment mismatch"
    );
}

/// Verifier test: verify transfer_received (accept) with pre-generated vectors
#[test]
fn verify_transfer_received_with_vectors() {
    use confidential_assets_primitives::ZkVerifier;
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek::traits::Identity;

    let asset_id = ASSET_ID_BYTES;
    let pk_receiver_pt = CompressedRistretto(RECEIVER_PK32)
        .decompress()
        .expect("pk_r");

    // Receiver phase: accept pending transfer
    let avail_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();
    let pending_old_c = CompressedRistretto(TRANSFER_DELTA_COMM_32)
        .decompress()
        .expect("ΔC");

    let pending_commits: Vec<[u8; 32]> = vec![pending_old_c.compress().to_bytes()];

    let result = <TestVerifier as ZkVerifier>::verify_transfer_received(
        asset_id,
        &pk_receiver_pt.compress().to_bytes(),
        &avail_old_c.compress().to_bytes(),
        &pending_old_c.compress().to_bytes(),
        &pending_commits,
        ACCEPT_ENVELOPE,
    );

    assert!(result.is_ok(), "verify_transfer_received should succeed");

    let (avail_new_bytes, pending_new_bytes) = result.unwrap();

    assert_eq!(
        avail_new_bytes.as_slice(),
        &ACCEPT_AVAIL_NEW_COMM_32,
        "avail_new commitment mismatch"
    );
    assert_eq!(
        pending_new_bytes.as_slice(),
        &ACCEPT_PENDING_NEW_COMM_32,
        "pending_new commitment mismatch"
    );
}

/// Verifier test: verify mint with pre-generated vectors
#[test]
fn verify_mint_with_vectors() {
    use confidential_assets_primitives::{PublicKeyBytes, ZkVerifier};
    use core::convert::TryFrom;

    let asset_id = ASSET_ID_BYTES;
    let to_pk_bv = PublicKeyBytes::try_from(RECEIVER_PK32.to_vec()).expect("pk bv");

    let result = <TestVerifier as ZkVerifier>::verify_mint(
        asset_id,
        &to_pk_bv,
        &[], // to_old_pending = identity
        &[], // total_old = identity
        MINT_PROOF,
    );

    assert!(result.is_ok(), "verify_mint should succeed");

    let (to_new_bytes, total_new_bytes, minted_ct_bytes) = result.unwrap();

    assert_eq!(minted_ct_bytes, MINTED_CT_64, "minted_ct mismatch");
    assert_eq!(
        to_new_bytes.as_slice(),
        &MINT_TO_NEW_COMM_32,
        "to_new commitment mismatch"
    );
    assert_eq!(
        total_new_bytes.as_slice(),
        &MINT_TOTAL_NEW_COMM_32,
        "total_new commitment mismatch"
    );
}

/// Verifier test: verify burn with pre-generated vectors
#[test]
fn verify_burn_with_vectors() {
    use confidential_assets_primitives::{EncryptedAmount, PublicKeyBytes, ZkVerifier};
    use core::convert::TryFrom;
    use curve25519_dalek::ristretto::CompressedRistretto;

    let asset_id = ASSET_ID_BYTES;
    let from_pk_bv = PublicKeyBytes::try_from(SENDER_PK32.to_vec()).expect("pk bv");
    let from_old_c = CompressedRistretto(BURN_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let total_old_c = CompressedRistretto(BURN_TOTAL_OLD_COMM_32)
        .decompress()
        .expect("total_old");

    let amount_ct_bv = EncryptedAmount::try_from(BURN_AMOUNT_CT_64.to_vec()).expect("ct bv");

    let result = <TestVerifier as ZkVerifier>::verify_burn(
        asset_id,
        &from_pk_bv,
        &from_old_c.compress().to_bytes(),
        &total_old_c.compress().to_bytes(),
        &amount_ct_bv,
        BURN_PROOF,
    );

    assert!(result.is_ok(), "verify_burn should succeed");

    let (from_new_bytes, total_new_bytes, disclosed) = result.unwrap();

    // Vector burned 120 units
    assert_eq!(disclosed, 120u64, "disclosed amount mismatch");
    assert_eq!(
        from_new_bytes.as_slice(),
        &BURN_FROM_NEW_COMM_32,
        "from_new commitment mismatch"
    );
    assert_eq!(
        total_new_bytes.as_slice(),
        &BURN_TOTAL_NEW_COMM_32,
        "total_new commitment mismatch"
    );
}

/// Full transfer round-trip test using vectors
#[test]
fn full_transfer_roundtrip_with_vectors() {
    use confidential_assets_primitives::ZkVerifier;
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek::traits::Identity;

    let asset_id = ASSET_ID_BYTES;

    // === Phase 1: Sender sends transfer ===
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();

    let (from_new_bytes, to_new_pending_bytes) =
        <TestVerifier as ZkVerifier>::verify_transfer_sent(
            asset_id,
            &SENDER_PK32,
            &RECEIVER_PK32,
            &from_old_c.compress().to_bytes(),
            &to_old_c.compress().to_bytes(),
            &TRANSFER_DELTA_CT_64,
            TRANSFER_BUNDLE,
        )
        .expect("sender verify failed");

    // Verify intermediate state
    assert_eq!(from_new_bytes.as_slice(), &TRANSFER_FROM_NEW_COMM_32);
    assert_eq!(to_new_pending_bytes.as_slice(), &TRANSFER_TO_NEW_COMM_32);

    // === Phase 2: Receiver accepts ===
    let pk_receiver_pt = CompressedRistretto(RECEIVER_PK32)
        .decompress()
        .expect("pk_r");

    let avail_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();
    let pending_old_c = CompressedRistretto(TRANSFER_DELTA_COMM_32)
        .decompress()
        .expect("ΔC");

    let pending_commits: Vec<[u8; 32]> = vec![pending_old_c.compress().to_bytes()];

    let (avail_new_bytes, pending_new_bytes) =
        <TestVerifier as ZkVerifier>::verify_transfer_received(
            asset_id,
            &pk_receiver_pt.compress().to_bytes(),
            &avail_old_c.compress().to_bytes(),
            &pending_old_c.compress().to_bytes(),
            &pending_commits,
            ACCEPT_ENVELOPE,
        )
        .expect("receiver verify failed");

    // Verify final state
    assert_eq!(avail_new_bytes.as_slice(), &ACCEPT_AVAIL_NEW_COMM_32);
    assert_eq!(pending_new_bytes.as_slice(), &ACCEPT_PENDING_NEW_COMM_32);

    // === Verify invariants ===
    // After accept, receiver's available should equal the transferred delta commitment
    assert_eq!(
        &ACCEPT_AVAIL_NEW_COMM_32, &TRANSFER_DELTA_COMM_32,
        "Receiver available should match delta commitment after accept"
    );

    // Pending should be cleared (zero commitment)
    assert_eq!(
        ACCEPT_PENDING_NEW_COMM_32, [0u8; 32],
        "Pending should be zero after full accept"
    );
}

/// Test that tampered bundles are rejected
#[test]
fn tampered_bundle_rejected() {
    use confidential_assets_primitives::ZkVerifier;
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek::traits::Identity;

    let asset_id = ASSET_ID_BYTES;
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();

    // Tamper with the bundle
    let mut tampered_bundle = TRANSFER_BUNDLE.to_vec();
    assert!(
        tampered_bundle.len() > 50,
        "Test vector TRANSFER_BUNDLE must be longer than 50 bytes"
    );
    tampered_bundle[50] ^= 0xFF; // Flip some bits

    let result = <TestVerifier as ZkVerifier>::verify_transfer_sent(
        asset_id,
        &SENDER_PK32,
        &RECEIVER_PK32,
        &from_old_c.compress().to_bytes(),
        &to_old_c.compress().to_bytes(),
        &TRANSFER_DELTA_CT_64,
        &tampered_bundle,
    );

    assert!(result.is_err(), "Tampered bundle should be rejected");
}

/// Test that wrong public key causes verification failure
#[test]
fn wrong_pk_rejected() {
    use confidential_assets_primitives::ZkVerifier;
    use curve25519_dalek::ristretto::CompressedRistretto;
    use curve25519_dalek::traits::Identity;

    let asset_id = ASSET_ID_BYTES;
    let from_old_c = CompressedRistretto(TRANSFER_FROM_OLD_COMM_32)
        .decompress()
        .expect("from_old");
    let to_old_c = curve25519_dalek::ristretto::RistrettoPoint::identity();

    // Use wrong sender PK (swap sender and receiver)
    let result = <TestVerifier as ZkVerifier>::verify_transfer_sent(
        asset_id,
        &RECEIVER_PK32, // Wrong! Should be SENDER_PK32
        &SENDER_PK32,   // Wrong! Should be RECEIVER_PK32
        &from_old_c.compress().to_bytes(),
        &to_old_c.compress().to_bytes(),
        &TRANSFER_DELTA_CT_64,
        TRANSFER_BUNDLE,
    );

    assert!(result.is_err(), "Wrong public keys should cause failure");
}
