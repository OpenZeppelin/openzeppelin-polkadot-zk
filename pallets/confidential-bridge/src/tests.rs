use crate::{Error, Event, mock::*};
use confidential_assets_primitives::EncryptedAmount;
use frame_support::assert_ok;

// helpers
fn ct(b: u8) -> EncryptedAmount {
    [b; 64]
}
fn last_event() -> RuntimeEvent {
    frame_system::Pallet::<Runtime>::events()
        .pop()
        .expect("event")
        .event
}

#[test]
fn send_confidential_initiates_and_records_pending() {
    new_test_ext().execute_with(|| {
        // Prepare keys for sender and escrow receiver inside escrow pallet.
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);

        // dest_para must differ from SelfParaId (1) to avoid NoSelfBridge.
        let dest_para = 2u32;
        let asset = ASSET;
        let amount = ct(9);
        let lock_proof = proof(&[1, 2, 3]);
        let accept_envelope = proof(&[4, 5, 6]);

        assert_ok!(ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            dest_para,
            BOB, // dest account on the other chain
            asset,
            amount,
            lock_proof,
            accept_envelope.clone(),
        ));

        // Event: OutboundTransferInitiated with id 0 (first transfer), asset.
        match last_event() {
            RuntimeEvent::ConfidentialBridge(Event::OutboundTransferInitiated {
                id,
                from,
                dest_para: dp,
                asset: ev_asset,
            }) => {
                assert_eq!(id, 0);
                assert_eq!(from, ALICE);
                assert_eq!(dp, dest_para);
                assert_eq!(ev_asset, asset);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        // Pending record is stored with correct fields.
        let rec = ConfidentialBridge::pending(0).expect("pending exists");
        assert_eq!(rec.from, ALICE);
        assert_eq!(rec.dest_para, dest_para);
        assert_eq!(rec.dest_account, BOB);
        assert_eq!(rec.asset, asset);
        assert_eq!(rec.encrypted_amount, amount);
        // Deadline = block 1 + DefaultTimeout(10) = 11
        assert_eq!(rec.deadline, 11);
        assert!(!rec.completed);
    });
}

#[test]
fn send_confidential_rejects_self_bridge() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);

        // SelfParaId in mock is ConstU32<1>
        let err = ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            1, // self
            BOB,
            ASSET,
            ct(1),
            proof(&[]),
            proof(&[]),
        )
        .unwrap_err();

        assert_eq!(err, Error::<Runtime>::NoSelfBridge.into());
    });
}

#[test]
fn confirm_success_releases_to_burn_and_burns_then_clears_pending() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{NextPendingDepositId, PendingBalanceCommit, PendingDeposits};

        // Setup: keys for ALICE (sender), escrow account (to hold), and burn account (to receive & burn).
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);
        let burn_acc = ConfidentialBridge::burn_account();
        set_pk(burn_acc);

        // First, create a pending transfer via send_confidential (id = 0).
        assert_ok!(ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            2,
            BOB,
            ASSET,
            ct(7),
            proof(&[1]),
            proof(&[2, 2]),
        ));
        // Sanity
        assert!(ConfidentialBridge::pending(0).is_some());

        // Now confirm success as Root (XcmOrigin in mock is EnsureRoot)
        assert_ok!(ConfidentialBridge::confirm_success(
            RuntimeOrigin::root(),
            0,
            proof(&[9, 9]), // release_proof
            proof(&[8, 8]), // burn_proof
        ));

        // Event emitted
        match last_event() {
            RuntimeEvent::ConfidentialBridge(Event::OutboundTransferConfirmed { id, asset }) => {
                assert_eq!(id, 0);
                assert_eq!(asset, ASSET);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        // Pending cleared
        assert!(ConfidentialBridge::pending(0).is_none());

        // Effects visible in the backend:
        // - escrow_release moved ciphertext to burn account pending list, then
        // - burn_encrypted consumed it; with our AlwaysOkVerifier we can at least
        //   observe a pending commit created on release before burn.
        // Because burn does not clear pending in the mock, we assert that the burn
        // receiver saw a pending update and a UTXO got recorded at some point.
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, burn_acc).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((burn_acc, ASSET, 0)).unwrap(),
            ct(7)
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(burn_acc, ASSET), 1);
    });
}

#[test]
fn confirm_success_errors_when_not_found() {
    new_test_ext().execute_with(|| {
        // No pending transfer with id 99
        let err =
            ConfidentialBridge::confirm_success(RuntimeOrigin::root(), 99, proof(&[]), proof(&[]))
                .unwrap_err();

        assert_eq!(err, Error::<Runtime>::NotFound.into());
    });
}

#[test]
fn cancel_and_refund_by_sender_after_deadline() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{NextPendingDepositId, PendingBalanceCommit, PendingDeposits};

        // Prepare keys: ALICE (sender), escrow (holds), and ALICE to receive refund.
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);

        // Create pending transfer id 0.
        assert_ok!(ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            2,
            BOB,
            ASSET,
            ct(44),
            proof(&[1]),
            proof(&[2]),
        ));
        let rec = ConfidentialBridge::pending(0).unwrap();
        assert_eq!(rec.deadline, 11);

        // Advance to >= deadline
        frame_system::Pallet::<Runtime>::set_block_number(12);

        assert_ok!(ConfidentialBridge::cancel_and_refund(
            RuntimeOrigin::signed(ALICE),
            0,
            proof(&[7, 7]), // refund proof used by escrow_refund
        ));

        // Event
        match last_event() {
            RuntimeEvent::ConfidentialBridge(Event::OutboundTransferRefunded { id, asset }) => {
                assert_eq!(id, 0);
                assert_eq!(asset, ASSET);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        // Pending cleared
        assert!(ConfidentialBridge::pending(0).is_none());

        // Backend effects: refund to ALICE pending + UTXO(0).
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, ALICE).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((ALICE, ASSET, 0)).unwrap(),
            ct(44)
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(ALICE, ASSET), 1);
    });
}

#[test]
fn cancel_and_refund_by_root_before_deadline() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);

        assert_ok!(ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            2,
            BOB,
            ASSET,
            ct(10),
            proof(&[1]),
            proof(&[2]),
        ));

        // Before deadline, but root is allowed to cancel.
        assert_ok!(ConfidentialBridge::cancel_and_refund(
            RuntimeOrigin::root(),
            0,
            proof(&[3, 3]),
        ));

        match last_event() {
            RuntimeEvent::ConfidentialBridge(Event::OutboundTransferRefunded { id, asset }) => {
                assert_eq!(id, 0);
                assert_eq!(asset, ASSET);
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert!(ConfidentialBridge::pending(0).is_none());
    });
}

#[test]
fn cancel_and_refund_errors_when_not_sender_or_not_expired() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        let escrow_acc = ConfidentialEscrow::escrow_account();
        set_pk(escrow_acc);

        assert_ok!(ConfidentialBridge::send_confidential(
            RuntimeOrigin::signed(ALICE),
            2,
            BOB,
            ASSET,
            ct(3),
            proof(&[1]),
            proof(&[2]),
        ));

        // Wrong caller (BOB), not privileged → NotSender
        let err = ConfidentialBridge::cancel_and_refund(RuntimeOrigin::signed(BOB), 0, proof(&[9]))
            .unwrap_err();
        assert_eq!(err, Error::<Runtime>::NotSender.into());

        // Right caller (ALICE) but before deadline → NotExpired
        let err =
            ConfidentialBridge::cancel_and_refund(RuntimeOrigin::signed(ALICE), 0, proof(&[9]))
                .unwrap_err();
        assert_eq!(err, Error::<Runtime>::NotExpired.into());
    });
}

#[test]
fn receive_confidential_mints_on_incoming_packet() {
    new_test_ext().execute_with(|| {
        use parity_scale_codec::Encode;

        // Destination will mint for BOB; need BOB's PK for backend mint.
        set_pk(BOB);

        // Build payload without importing BridgePacket:
        // SCALE for struct = ordered fields, same as tuple encoding.
        let payload = (0u64, BOB, ASSET, ct(55), proof(&[1, 2, 3])).encode();
        let bounded: sp_runtime::BoundedVec<u8, sp_runtime::traits::ConstU32<1024>> =
            payload.clone().try_into().expect("fits");

        assert_ok!(ConfidentialBridge::receive_confidential(
            RuntimeOrigin::root(),
            bounded,
        ));

        match last_event() {
            RuntimeEvent::ConfidentialBridge(Event::InboundTransferExecuted {
                id,
                asset,
                minted,
            }) => {
                assert_eq!(id, 0);
                assert_eq!(asset, ASSET);
                assert_eq!(minted, [5u8; 64]); // AlwaysOkVerifier::verify_mint
            }
            other => panic!("unexpected event: {other:?}"),
        }
    });
}
