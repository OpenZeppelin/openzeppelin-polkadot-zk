// Confidential XCM Tests
use crate::*;

use confidential_assets_primitives::ConfidentialBackend;
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT as G, scalar::Scalar, traits::Identity,
};
use frame_support::{assert_ok, weights::Weight};
use parity_scale_codec::Encode;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use xcm_simulator::TestExt;
use zkhe_prover::{
    BurnInput, MintInput, ReceiverAcceptInput, SenderInput, prove_burn, prove_mint,
    prove_receiver_accept, prove_sender_transfer,
};

fn asset_id_bytes_u128(id: u128) -> Vec<u8> {
    id.to_le_bytes().to_vec()
} // verifier pads to 32B

// ---------------- helpers ----------------
fn h() -> curve25519_dalek::ristretto::RistrettoPoint {
    use sha2::Sha512;
    curve25519_dalek::ristretto::RistrettoPoint::hash_from_bytes::<Sha512>(b"Zether/PedersenH")
}
fn p32(pt: &curve25519_dalek::ristretto::RistrettoPoint) -> [u8; 32] {
    pt.compress().to_bytes()
}
fn pbytes(label: &str, bytes: &[u8]) {
    println!("{label} (len={}): 0x{}", bytes.len(), hex::encode(bytes));
}
fn print_events_para_a() {
    for e in parachain::System::events() {
        println!("ParaA event: {:?}", e.event);
    }
}
fn print_events_para_b() {
    for e in parachain::System::events() {
        println!("ParaB event: {:?}", e.event);
    }
}
fn expect_event_or_dump<E: core::fmt::Debug>(
    ok: Result<(), E>,
    phase: &str,
    on_dump_a: impl FnOnce(),
    on_dump_b: impl FnOnce(),
) {
    if ok.is_err() {
        println!("--- {phase}: FAILURE, dumping events ---");
        on_dump_a();
        on_dump_b();
    }
    assert!(ok.is_ok(), "Expected Ok(_). Got Err({ok:?})");
}

// For storage debugging (best-effort; ignore if not present)
fn show_pk(label: &str, who: &parachain::AccountId) {
    let pk_opt = parachain::Zkhe::public_key(who);
    match pk_opt {
        Some(bv) => println!("{label} PK in storage: {:?}", bv),
        None => println!("{label} PK in storage: <none>"),
    }
}
fn show_avail_commit(label: &str, asset: u128, who: parachain::AccountId) {
    use pallet_zkhe::AvailableBalanceCommit;
    let got = AvailableBalanceCommit::<parachain::Runtime>::get(asset, who);
    match got {
        Some(c) => println!("{label} available commit: {:?}", c),
        None => println!("{label} available commit: <none>"),
    }
}
fn show_pending_commit(label: &str, asset: u128, who: parachain::AccountId) {
    use pallet_zkhe::PendingBalanceCommit;
    let got = PendingBalanceCommit::<parachain::Runtime>::get(asset, who);
    match got {
        Some(c) => println!("{label} pending commit: {:?}", c),
        None => println!("{label} pending commit: <none>"),
    }
}

/// Demonstrates confidential xcm transfers via pallet-confidential-bridge:
/// 1. Send confidential assets from source parachain to dest parachain + escrow local confidential assets
/// 2. Claim confidential assets on dest parachain + send confirmation back to source parachain
/// 3. Release escrow on source parachain once received confirmed execution on dest parachain
#[test]
fn confidential_xcm_transfer() {
    MockNet::reset();

    let asset_id_u128 = 0u128;
    let asset_id = asset_id_bytes_u128(asset_id_u128);
    let network_id = [0u8; 32];

    let from_old_v: u64 = 1_234;
    let from_old_r: u64 = 42;

    let sk_sender = Scalar::from(5u64);
    let pk_sender = sk_sender * G;
    let sk_receiver = Scalar::from(9u64);
    let pk_receiver = sk_receiver * G;

    let dv: u64 = 111;

    let mut seed = [0u8; 32];
    seed[0] = 7;

    // Pre-compute ΔC from the same seed the sender prover uses (for debug)
    let mut chacha = ChaCha20Rng::from_seed(seed);
    let _k_for_ct = chacha.next_u64();
    let delta_rho = Scalar::from(chacha.next_u64());
    let delta_comm = Scalar::from(dv) * G + delta_rho * h();
    println!(
        "Re-derived Δrho: {}",
        hex::encode(Scalar::from(delta_rho).to_bytes())
    );
    println!("Re-derived ΔC:   {}", hex::encode(p32(&delta_comm)));

    // ============ Phase 0 (ParaB): ensure receiver PK ============
    ParaB::execute_with(|| {
        let pk_bob = pk_receiver.compress().to_bytes().to_vec();
        assert_ok!(parachain::Zkhe::set_public_key(
            &BOB,
            &pk_bob.clone().try_into().unwrap()
        ));
        show_pk("ParaB/BOB", &BOB);
        print_events_para_b();
    });

    // ============ Phase 1 (ParaA): send_confidential ============
    ParaA::execute_with(|| {
        // Install required PKs
        let pk_sender_bytes = pk_sender.compress().to_bytes().to_vec();
        assert_ok!(parachain::Zkhe::set_public_key(
            &ALICE,
            &pk_sender_bytes.clone().try_into().unwrap()
        ));
        let escrow = parachain::ConfidentialEscrow::escrow_account();
        let burn = parachain::ConfidentialBridge::burn_account();
        let dummy_pk = pk_receiver
            .compress()
            .to_bytes()
            .to_vec()
            .try_into()
            .unwrap();
        let _ = parachain::Zkhe::set_public_key(&escrow, &dummy_pk);
        let _ = parachain::Zkhe::set_public_key(&burn, &dummy_pk);

        // Seed ALICE available commitment (must match prover input)
        let from_old_c = Scalar::from(from_old_v) * G + Scalar::from(from_old_r) * h();
        pallet_zkhe::AvailableBalanceCommit::<parachain::Runtime>::insert(
            asset_id_u128,
            ALICE,
            p32(&from_old_c),
        );

        // Diagnostics before proving
        println!("=== Phase 1 pre-prove diagnostics (ParaA) ===");
        show_pk("ALICE", &ALICE);
        show_pk("ESCROW", &escrow);
        show_pk("BURN", &burn);
        show_avail_commit("ALICE avail (should be from_old_c)", asset_id_u128, ALICE);
        show_pending_commit("ALICE pending", asset_id_u128, ALICE);
        println!("Expected from_old_c: {}", hex::encode(p32(&from_old_c)));
        println!(
            "Sender pk (expected): {}",
            hex::encode(pk_sender.compress().to_bytes())
        );

        // Sender proof
        let s_in = SenderInput {
            asset_id: asset_id.clone(),
            network_id,
            sender_pk: pk_sender,
            receiver_pk: pk_receiver,
            from_old_c,
            from_old_opening: (from_old_v, Scalar::from(from_old_r)),
            to_old_c: curve25519_dalek::ristretto::RistrettoPoint::identity(),
            delta_value: dv,
            rng_seed: seed,
            fee_c: None,
        };
        let s_out = prove_sender_transfer(&s_in).expect("sender prover");
        pbytes("delta_ct_bytes", &s_out.delta_ct_bytes);
        println!(
            "sender_bundle_bytes.len={}",
            s_out.sender_bundle_bytes.len()
        );
        println!("delta_comm_bytes={}", hex::encode(&s_out.delta_comm_bytes));
        println!("from_new_c={}", hex::encode(&s_out.from_new_c));
        println!("to_new_c={}", hex::encode(&s_out.to_new_c));

        // Destination mint proof (for ParaB)
        let mint_seed = {
            let mut s = [0u8; 32];
            s[0] = 0xA5;
            s
        };
        let m_in = MintInput {
            asset_id: asset_id.clone(),
            network_id,
            to_pk: pk_receiver,
            to_pending_old_c: curve25519_dalek::ristretto::RistrettoPoint::identity(),
            to_pending_old_opening: (0u64, Scalar::from(0u64)),
            total_old_c: curve25519_dalek::ristretto::RistrettoPoint::identity(),
            total_old_opening: (0u64, Scalar::from(0u64)),
            mint_value: dv,
            rng_seed: mint_seed,
        };
        let m_out = prove_mint(&m_in).expect("mint prover");
        println!("mint.proof_bytes.len={}", m_out.proof_bytes.len());

        // Call
        let call_res = parachain::ConfidentialBridge::send_confidential(
            parachain::RuntimeOrigin::signed(ALICE),
            2,
            BOB,
            asset_id_u128,
            s_out.delta_ct_bytes,
            s_out
                .sender_bundle_bytes
                .clone()
                .try_into()
                .expect("bundle→BoundedVec"),
            m_out
                .proof_bytes
                .clone()
                .try_into()
                .expect("mint→BoundedVec"),
        );

        if call_res.is_err() {
            println!("escrow failed");
            print_events_para_a();
            // Storage snapshots for quick diff
            show_avail_commit("ALICE avail (post-error)", asset_id_u128, ALICE);
            show_pending_commit("ESCROW pending (if any)", asset_id_u128, escrow);
        }
        expect_event_or_dump(
            call_res.map(|_| ()),
            "Phase 1 send_confidential",
            || print_events_para_a(),
            || ParaB::execute_with(|| print_events_para_b()),
        );
    });

    // ============ Phase 2 (ParaB): inbound minted ============
    ParaB::execute_with(|| {
        println!("=== Phase 2 diagnostics (ParaB after inbound) ===");
        print_events_para_b();
        let ok = parachain::System::events().iter().any(|e| {
            matches!(
                e.event,
                parachain::RuntimeEvent::ConfidentialBridge(
                    pallet_confidential_bridge::Event::InboundTransferExecuted { id: 0, .. }
                )
            )
        });
        if !ok {
            // Dump storage hints
            show_pk("BOB", &BOB);
        }
        assert!(ok, "expected InboundTransferExecuted on ParaB");
    });

    // Prepare burn account PK
    let burn_sk = Scalar::from(777u64);
    let burn_pk = burn_sk * G;

    ParaA::execute_with(|| {
        let burn_acc = parachain::ConfidentialBridge::burn_account();
        let burn_pk_bytes: Vec<u8> = burn_pk.compress().to_bytes().to_vec();
        assert_ok!(parachain::Zkhe::set_public_key(
            &burn_acc,
            &burn_pk_bytes.try_into().unwrap()
        ));
        println!("=== Phase 3 pre-confirm (ParaA) ===");
        show_pk("BURN", &burn_acc);
        print_events_para_a();
    });

    // ============ Phase 3 (ParaB): build release+burn proofs, send confirm_success ============
    ParaB::execute_with(|| {
        // Re-derive Δρ and ΔC for receiver-accept/burn
        let mut chacha = ChaCha20Rng::from_seed({
            let mut s = [0u8; 32];
            s[0] = 7;
            s
        });
        let _k_ignore = chacha.next_u64();
        let delta_rho = Scalar::from(chacha.next_u64());
        let delta_comm = Scalar::from(dv) * G + delta_rho * h();

        // Burn PK (on A)
        let burn_pk = {
            let sk = Scalar::from(777u64);
            sk * G
        };

        // Release proof for burn account’s accept (avail += Δ, pending -= Δ)
        let a_release_in = ReceiverAcceptInput {
            asset_id: asset_id.clone(),
            network_id,
            receiver_pk: burn_pk,
            avail_old_c: curve25519_dalek::ristretto::RistrettoPoint::identity(),
            avail_old_opening: (0u64, Scalar::from(0u64)),
            pending_old_c: delta_comm,
            pending_old_opening: (dv, delta_rho),
            delta_comm,
            delta_value: dv,
            delta_rho,
        };
        let a_release_out = prove_receiver_accept(&a_release_in).expect("release prover");
        println!(
            "release.accept_envelope.len={}",
            a_release_out.accept_envelope.len()
        );

        // Burn proof
        let b_burn_in = BurnInput {
            asset_id: asset_id.clone(),
            network_id,
            from_pk: burn_pk,
            from_avail_old_c: delta_comm,
            from_avail_old_opening: (dv, delta_rho),
            total_old_c: delta_comm,
            total_old_opening: (dv, delta_rho),
            burn_value: dv,
            rng_seed: {
                let mut s = [0u8; 32];
                s[1] = 0x5C;
                s
            },
        };
        let b_burn_out = prove_burn(&b_burn_in).expect("burn prover");
        println!("burn.proof_bytes.len={}", b_burn_out.proof_bytes.len());

        // Ship both proofs back to A *with an XCM origin* so EnsureXcmOrigin passes
        let call = parachain::RuntimeCall::ConfidentialBridge(pallet_confidential_bridge::Call::<
            parachain::Runtime,
        >::confirm_success {
            id: 0,
            release_proof: a_release_out.accept_envelope.clone().try_into().unwrap(),
            burn_proof: b_burn_out.proof_bytes.clone().try_into().unwrap(),
        });

        // Destination is Parent -> Parachain(1) (ParaA)
        let dest = Location::new(1, [Parachain(1u32.into())]);

        // IMPORTANT: use OriginKind::Xcm instead of SovereignAccount
        let msg = Xcm(vec![Transact {
            origin_kind: OriginKind::Xcm,
            fallback_max_weight: Some(Weight::from_parts(3_000_000_000, 0)),
            call: call.encode().into(),
        }]);

        // Send with the same router you already use elsewhere
        let origin =
            parachain::RuntimeOrigin::signed(parachain::ConfidentialBridge::burn_account());
        // Send via PolkadotXcm::send (same path as your XcmHrmpMessenger)
        let res = parachain::PolkadotXcm::send(
            origin,
            Box::new(VersionedLocation::from(dest)),
            Box::new(VersionedXcm::from(msg)),
        );
        if res.is_err() {
            println!("confirm_success XCM send failed on ParaB: {res:?}");
            print_events_para_b();
        }
        assert_ok!(res);
        print_events_para_b();
    });
}
