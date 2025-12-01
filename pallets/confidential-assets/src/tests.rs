use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use proptest::prelude::*;

// Small helpers
fn ct(x: u8) -> EncryptedAmount {
    [x; 64]
}
fn last_event() -> RuntimeEvent {
    frame_system::Pallet::<Runtime>::events()
        .pop()
        .expect("event")
        .event
}

#[test]
fn set_public_key_emits_event() {
    new_test_ext().execute_with(|| {
        let pk: PublicKeyBytes = vec![9u8; 32].try_into().unwrap();
        assert_ok!(ConfidentialAssets::set_public_key(
            RuntimeOrigin::signed(ALICE),
            pk
        ));

        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::PublicKeySet { who }) => {
                assert_eq!(who, ALICE);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn deposit_calls_ramp_then_backend_and_emits_deposited() {
    new_test_ext().execute_with(|| {
        // Backend needs a pk for the recipient account.
        set_pk(ALICE);

        let proof = proof(&[1, 2, 3]);
        let amount: Balance = 1_000;

        assert_ok!(ConfidentialAssets::deposit(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            amount,
            proof
        ));

        // Event reflects ramp burn + backend mint_encrypted returning [5;64].
        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::Deposited {
                who,
                asset,
                amount: ev_amount,
                encrypted_amount,
            }) => {
                assert_eq!(who, ALICE);
                assert_eq!(asset, ASSET);
                assert_eq!(ev_amount, amount);
                assert_eq!(encrypted_amount, [5u8; 64]);
            }
            e => panic!("unexpected event: {e:?}"),
        }

        // Read helpers surface backend state (mock returns constants).
        assert_eq!(
            ConfidentialAssets::confidential_total_supply(ASSET),
            [11u8; 32]
        );
    });
}

#[test]
fn withdraw_debits_confidential_then_mints_public_and_emits_withdrawn() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);

        let enc = ct(77);
        let proof = proof(&[9, 9]);

        assert_ok!(ConfidentialAssets::withdraw(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            enc,
            proof
        ));

        // Mock backend returns disclosed amount 42 and commits [20;32]/[21;32]
        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::Withdrawn {
                who,
                asset,
                encrypted_amount,
                amount,
            }) => {
                assert_eq!(who, ALICE);
                assert_eq!(asset, ASSET);
                assert_eq!(encrypted_amount, enc);
                assert_eq!(amount, 42u64);
            }
            e => panic!("unexpected event: {e:?}"),
        }

        // Helper reflects total supply commit from mock burn path.
        assert_eq!(
            ConfidentialAssets::confidential_total_supply(ASSET),
            [21u8; 32]
        );
    });
}

#[test]
fn confidential_transfer_updates_via_backend_and_emits() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        set_pk(BOB);

        let delta = ct(1);
        let proof = proof(&[7]);

        assert_ok!(ConfidentialAssets::confidential_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            BOB,
            delta,
            proof
        ));

        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::ConfidentialTransfer {
                asset,
                from,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(from, ALICE);
                assert_eq!(to, BOB);
                // Zkhe::transfer_encrypted returns the same ciphertext given
                assert_eq!(encrypted_amount, delta);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn disclose_amount_emits_event_with_mock_amount() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);

        assert_ok!(ConfidentialAssets::disclose_amount(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            ct(9)
        ));

        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::AmountDisclosed {
                asset,
                encrypted_amount,
                amount,
                discloser,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(encrypted_amount, ct(9));
                assert_eq!(amount, 123u64); // AlwaysOkVerifier::disclose
                assert_eq!(discloser, ALICE);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn confidential_claim_consumes_backend_utxos_and_emits() {
    new_test_ext().execute_with(|| {
        use pallet_zkhe::{
            AvailableBalanceCommit, NextPendingDepositId, PendingBalanceCommit, PendingDeposits,
        };

        set_pk(ALICE);

        // Seed a pending UTXO for the caller in the ZkHE backend.
        PendingDeposits::<Runtime>::insert((ALICE, ASSET, 0), ct(55));
        NextPendingDepositId::<Runtime>::insert(ALICE, ASSET, 1);

        // IDs=[0], opaque rest empty; mock verify_transfer_received => avail=[3;32], pending=[0;32]
        let input = accept_input(&[0], &[]);

        assert_ok!(ConfidentialAssets::confidential_claim(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            input
        ));

        // Storage effects in backend:
        assert_eq!(
            AvailableBalanceCommit::<Runtime>::get(ASSET, ALICE).unwrap(),
            [3u8; 32]
        );
        // pending_new == zero => pallet_zkhe removes PendingBalanceCommit
        assert!(PendingBalanceCommit::<Runtime>::get(ASSET, ALICE).is_none());
        assert!(PendingDeposits::<Runtime>::get((ALICE, ASSET, 0)).is_none());

        // Event surface:
        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::ConfidentialClaimed {
                asset,
                who,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(who, ALICE);
                // ZkHE::claim_encrypted returns [0;64] "no new UTXO" marker
                assert_eq!(encrypted_amount, [0u8; 64]);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn confidential_transfer_from_succeeds_when_caller_is_owner() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        set_pk(BOB);

        let delta = ct(3);

        // Caller == from => allowed even with Operators = ()
        assert_ok!(ConfidentialAssets::confidential_transfer_from(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            ALICE,
            BOB,
            delta,
            proof(&[])
        ));

        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::ConfidentialTransfer {
                asset,
                from,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(from, ALICE);
                assert_eq!(to, BOB);
                assert_eq!(encrypted_amount, delta);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn confidential_transfer_from_fails_when_caller_not_owner_and_not_operator() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        set_pk(BOB);

        // CHARLIE is not ALICE and Operators=() returns false
        let err = ConfidentialAssets::confidential_transfer_from(
            RuntimeOrigin::signed(CHARLIE),
            ASSET,
            ALICE,
            BOB,
            ct(4),
            proof(&[]),
        )
        .unwrap_err();

        assert_eq!(err, pallet::Error::<Runtime>::NotAuthorized.into());
    });
}

#[test]
fn confidential_transfer_acl_allows_any_caller_when_acl_is_unit() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        set_pk(BOB);

        // Acl = () authorizes all Ops; caller can be unrelated.
        assert_ok!(ConfidentialAssets::confidential_transfer_acl(
            RuntimeOrigin::signed(CHARLIE),
            ASSET,
            ALICE,
            BOB,
            ct(5),
            proof(&[])
        ));

        match last_event() {
            RuntimeEvent::ConfidentialAssets(pallet::Event::ConfidentialTransfer {
                asset,
                from,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(from, ALICE);
                assert_eq!(to, BOB);
                assert_eq!(encrypted_amount, ct(5));
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

// ===================== PROPERTY TESTS =====================

prop_compose! {
    /// Generate arbitrary account IDs (non-zero)
    fn arb_account()(id in 1u64..1000) -> AccountId {
        id
    }
}

prop_compose! {
    /// Generate arbitrary asset IDs
    fn arb_asset()(id in 1u32..100) -> AssetId {
        id
    }
}

prop_compose! {
    /// Generate arbitrary balance amounts
    fn arb_balance()(amount in 1u64..1_000_000) -> Balance {
        amount
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: set_public_key always succeeds and emits event
    #[test]
    fn prop_set_public_key_always_succeeds(
        who in arb_account(),
        pk_fill in any::<u8>()
    ) {
        new_test_ext().execute_with(|| {
            let pk: PublicKeyBytes = vec![pk_fill; 32].try_into().unwrap();

            let result = ConfidentialAssets::set_public_key(
                RuntimeOrigin::signed(who),
                pk
            );

            prop_assert!(result.is_ok(), "set_public_key should always succeed");

            // Verify event was emitted
            let events = frame_system::Pallet::<Runtime>::events();
            let has_pk_event = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::ConfidentialAssets(pallet::Event::PublicKeySet { who: w }) if w == who
                )
            });
            prop_assert!(has_pk_event, "PublicKeySet event should be emitted");

            Ok(())
        })?;
    }

    /// Property: deposit with valid PK succeeds and emits Deposited event
    #[test]
    fn prop_deposit_succeeds_with_pk(
        who in arb_account(),
        asset in arb_asset(),
        amount in arb_balance()
    ) {
        new_test_ext().execute_with(|| {
            set_pk(who);

            let result = ConfidentialAssets::deposit(
                RuntimeOrigin::signed(who),
                asset,
                amount,
                proof(&[1, 2, 3])
            );

            prop_assert!(result.is_ok(), "deposit should succeed: {:?}", result);

            // Verify event was emitted
            let events = frame_system::Pallet::<Runtime>::events();
            let has_deposit_event = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::ConfidentialAssets(pallet::Event::Deposited {
                        who: w,
                        asset: a,
                        amount: am,
                        ..
                    }) if w == who && a == asset && am == amount
                )
            });
            prop_assert!(has_deposit_event, "Deposited event should be emitted");

            Ok(())
        })?;
    }

    /// Property: confidential_transfer between distinct accounts succeeds
    #[test]
    fn prop_confidential_transfer_succeeds(
        from in arb_account(),
        to in arb_account(),
        asset in arb_asset(),
        ct_fill in any::<u8>()
    ) {
        let to = if to == from { to + 1 } else { to };

        new_test_ext().execute_with(|| {
            set_pk(from);
            set_pk(to);

            let delta = ct(ct_fill);

            let result = ConfidentialAssets::confidential_transfer(
                RuntimeOrigin::signed(from),
                asset,
                to,
                delta,
                proof(&[])
            );

            prop_assert!(result.is_ok(), "confidential_transfer should succeed: {:?}", result);

            // Verify event was emitted
            let events = frame_system::Pallet::<Runtime>::events();
            let has_transfer_event = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::ConfidentialAssets(pallet::Event::ConfidentialTransfer {
                        asset: a,
                        from: f,
                        to: t,
                        ..
                    }) if a == asset && f == from && t == to
                )
            });
            prop_assert!(has_transfer_event, "ConfidentialTransfer event should be emitted");

            Ok(())
        })?;
    }

    /// Property: withdraw succeeds and emits Withdrawn event with disclosed amount
    #[test]
    fn prop_withdraw_succeeds(
        who in arb_account(),
        asset in arb_asset(),
        ct_fill in any::<u8>()
    ) {
        new_test_ext().execute_with(|| {
            set_pk(who);

            let result = ConfidentialAssets::withdraw(
                RuntimeOrigin::signed(who),
                asset,
                ct(ct_fill),
                proof(&[])
            );

            prop_assert!(result.is_ok(), "withdraw should succeed: {:?}", result);

            // Mock verifier discloses 42
            let events = frame_system::Pallet::<Runtime>::events();
            let has_withdraw_event = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::ConfidentialAssets(pallet::Event::Withdrawn {
                        who: w,
                        asset: a,
                        amount: 42,
                        ..
                    }) if w == who && a == asset
                )
            });
            prop_assert!(has_withdraw_event, "Withdrawn event should be emitted with disclosed amount");

            Ok(())
        })?;
    }

    /// Property: disclose_amount succeeds and emits correct event
    #[test]
    fn prop_disclose_amount_succeeds(
        who in arb_account(),
        asset in arb_asset(),
        ct_fill in any::<u8>()
    ) {
        new_test_ext().execute_with(|| {
            set_pk(who);

            let result = ConfidentialAssets::disclose_amount(
                RuntimeOrigin::signed(who),
                asset,
                ct(ct_fill)
            );

            prop_assert!(result.is_ok(), "disclose_amount should succeed: {:?}", result);

            // Mock verifier discloses 123
            let events = frame_system::Pallet::<Runtime>::events();
            let has_disclose_event = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::ConfidentialAssets(pallet::Event::AmountDisclosed {
                        asset: a,
                        amount: 123,
                        discloser: d,
                        ..
                    }) if a == asset && d == who
                )
            });
            prop_assert!(has_disclose_event, "AmountDisclosed event should be emitted");

            Ok(())
        })?;
    }

    /// Property: confidential_transfer_from fails when caller is not owner and not operator
    #[test]
    fn prop_transfer_from_unauthorized_fails(
        owner in arb_account(),
        attacker in arb_account(),
        recipient in arb_account(),
        asset in arb_asset()
    ) {
        // Ensure all accounts are distinct
        let attacker = if attacker == owner { attacker + 1 } else { attacker };
        let recipient = if recipient == owner || recipient == attacker { recipient + 2 } else { recipient };

        new_test_ext().execute_with(|| {
            set_pk(owner);
            set_pk(attacker);
            set_pk(recipient);

            // Attacker tries to transfer from owner (not authorized since Operators = ())
            let result = ConfidentialAssets::confidential_transfer_from(
                RuntimeOrigin::signed(attacker),
                asset,
                owner,
                recipient,
                ct(1),
                proof(&[])
            );

            prop_assert!(result.is_err(), "Unauthorized transfer_from should fail");
            prop_assert_eq!(result.unwrap_err(), pallet::Error::<Runtime>::NotAuthorized.into());

            Ok(())
        })?;
    }

    /// Property: confidential_transfer_from succeeds when caller is owner
    #[test]
    fn prop_transfer_from_by_owner_succeeds(
        owner in arb_account(),
        recipient in arb_account(),
        asset in arb_asset()
    ) {
        let recipient = if recipient == owner { recipient + 1 } else { recipient };

        new_test_ext().execute_with(|| {
            set_pk(owner);
            set_pk(recipient);

            // Owner transfers from self (always allowed)
            let result = ConfidentialAssets::confidential_transfer_from(
                RuntimeOrigin::signed(owner),
                asset,
                owner,
                recipient,
                ct(1),
                proof(&[])
            );

            prop_assert!(result.is_ok(), "Owner transfer_from should succeed: {:?}", result);

            Ok(())
        })?;
    }
}
