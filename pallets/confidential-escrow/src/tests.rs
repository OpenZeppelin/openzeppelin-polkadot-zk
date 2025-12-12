use crate::{Error, Event, mock::*};
use confidential_assets_primitives::EncryptedAmount;
use frame_support::{assert_err, assert_ok};
use sp_runtime::traits::Zero;
// Avoid name clash: pallet alias = `ConfidentialEscrow`, trait aliased as CE.
use confidential_assets_primitives::ConfidentialEscrow as CE;

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
fn escrow_account_is_deterministic_and_nonzero() {
    new_test_ext().execute_with(|| {
        let acc = ConfidentialEscrow::escrow_account();
        assert!(!acc.is_zero());
        assert_ne!(acc, ALICE);
        assert_ne!(acc, BOB);
    });
}

#[test]
fn escrow_lock_moves_funds_to_escrow_and_emits_event() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{NextPendingDepositId, PendingBalanceCommit, PendingDeposits};

        // Need PKs for both sides (sender and escrow).
        set_pk(ALICE);
        let escrow = ConfidentialEscrow::escrow_account();
        set_pk(escrow);

        let delta = ct(11);
        let proof = proof(&[1, 2, 3]);

        assert_ok!(<ConfidentialEscrow as CE<AccountId, AssetId>>::escrow_lock(
            ASSET, &ALICE, delta, proof
        ));

        // Backend effects on ZkHE storage (receiver = escrow).
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, escrow).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((escrow, ASSET, 0)).unwrap(),
            delta
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(escrow, ASSET), 1);

        // Event surfaced by this pallet.
        match last_event() {
            RuntimeEvent::ConfidentialEscrow(Event::EscrowLocked {
                asset,
                from,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(from, ALICE);
                assert_eq!(encrypted_amount, delta);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    });
}

#[test]
fn escrow_release_moves_funds_from_escrow_to_beneficiary_and_emits_event() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{NextPendingDepositId, PendingBalanceCommit, PendingDeposits};

        let escrow = ConfidentialEscrow::escrow_account();
        set_pk(escrow);
        set_pk(BOB);

        let delta = ct(22);

        assert_ok!(
            <ConfidentialEscrow as CE<AccountId, AssetId>>::escrow_release(
                ASSET,
                &BOB,
                delta,
                proof(&[9])
            )
        );

        // Backend effects on receiver (BOB).
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, BOB).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((BOB, ASSET, 0)).unwrap(),
            delta
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(BOB, ASSET), 1);

        match last_event() {
            RuntimeEvent::ConfidentialEscrow(Event::EscrowReleased {
                asset,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(to, BOB);
                assert_eq!(encrypted_amount, delta);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    });
}

#[test]
fn escrow_refund_moves_funds_from_escrow_back_to_owner_and_emits_event() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{NextPendingDepositId, PendingBalanceCommit, PendingDeposits};

        let escrow = ConfidentialEscrow::escrow_account();
        set_pk(escrow);
        set_pk(ALICE);

        let delta = ct(33);

        assert_ok!(
            <ConfidentialEscrow as CE<AccountId, AssetId>>::escrow_refund(
                ASSET,
                &ALICE,
                delta,
                proof(&[4, 4])
            )
        );

        // Backend effects on receiver (ALICE).
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, ALICE).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((ALICE, ASSET, 0)).unwrap(),
            delta
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(ALICE, ASSET), 1);

        match last_event() {
            RuntimeEvent::ConfidentialEscrow(Event::EscrowRefunded {
                asset,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(to, ALICE);
                assert_eq!(encrypted_amount, delta);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    });
}

#[test]
fn escrow_lock_fails_with_backend_error_when_missing_public_key() {
    new_test_ext().execute_with(|| {
        // Only ALICE has a PK; ESCROW lacks a PK to receive -> backend error.
        set_pk(ALICE);
        let delta = ct(7);

        let res = <ConfidentialEscrow as CE<AccountId, AssetId>>::escrow_lock(
            ASSET,
            &ALICE,
            delta,
            proof(&[]),
        );

        assert_err!(res, Error::<Runtime>::BackendError);
    });
}
