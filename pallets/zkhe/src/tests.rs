use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use proptest::prelude::*;
use sp_runtime::traits::BadOrigin;

fn last_event() -> RuntimeEvent {
    frame_system::Pallet::<Runtime>::events()
        .pop()
        .expect("event")
        .event
}

// A 64B “ciphertext” convenience
fn ct(val: u8) -> EncryptedAmount {
    [val; 64]
}

#[test]
fn set_public_key_and_disclose_works() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);

        let amount = <Pallet<Runtime> as ConfidentialBackend<_, _, _>>::disclose_amount(
            ASSET,
            &ct(9),
            &ALICE,
        )
        .expect("ok");
        // Mock verifier discloses 123
        assert_eq!(amount, 123u64);
    });
}

#[test]
fn transfer_sets_commits_records_utxo_and_emits() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);
        set_pk(BOB);

        let delta = ct(99);
        let proof = proof(&[1, 2, 3]); // opaque to the pallet
        assert_ok!(Pallet::<Runtime>::transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            BOB,
            delta,
            proof
        ));

        // from_new_available = [1;32], to_new_pending = [2;32]
        assert_eq!(
            AvailableBalanceCommit::<Runtime>::get(ASSET, ALICE).unwrap(),
            [1u8; 32]
        );
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, BOB).unwrap(),
            [2u8; 32]
        );

        // A UTXO is recorded for the receiver at id 0
        assert_eq!(
            PendingDeposits::<Runtime>::get((BOB, ASSET, 0)).unwrap(),
            delta
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(BOB, ASSET), 1);

        // Event
        match last_event() {
            RuntimeEvent::Zkhe(pallet::Event::Transferred {
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
fn accept_pending_consumes_utxos_updates_balances_and_emits() {
    new_test_ext().execute_with(|| {
        set_pk(BOB);

        // Seed one pending deposit for BOB (id 0)
        PendingDeposits::<Runtime>::insert((BOB, ASSET, 0), ct(7));
        NextPendingDepositId::<Runtime>::insert(BOB, ASSET, 1);

        // Build envelope with ids=[0] and dummy rest
        let env = accept_input(&[0], &[9, 9, 9]);

        assert_ok!(Pallet::<Runtime>::accept_pending(
            RuntimeOrigin::signed(BOB),
            ASSET,
            env
        ));

        // Verifier returns avail_new=[3;32], pending_new=[0;32] -> pallet removes pending commit
        assert_eq!(
            AvailableBalanceCommit::<Runtime>::get(ASSET, BOB).unwrap(),
            [3u8; 32]
        );
        assert!(PendingBalanceCommit::<Runtime>::get(ASSET, BOB).is_none());

        // UTXO id 0 consumed
        assert!(PendingDeposits::<Runtime>::get((BOB, ASSET, 0)).is_none());

        // Event carries a “no new UTXO” marker (64 zeroes) by design of claim_encrypted()
        match last_event() {
            RuntimeEvent::Zkhe(pallet::Event::PendingAccepted {
                asset,
                who,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(who, BOB);
                assert_eq!(encrypted_amount, [0u8; 64]);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn accept_pending_and_transfer_chains_both_paths() {
    new_test_ext().execute_with(|| {
        set_pk(BOB);
        set_pk(CHARLIE);

        // Give BOB a pending UTXO id 0
        PendingDeposits::<Runtime>::insert((BOB, ASSET, 0), ct(55));
        NextPendingDepositId::<Runtime>::insert(BOB, ASSET, 1);

        let accept_env = accept_input(&[0], &[]); // ids + empty rest
        let transfer_proof = proof(&[1]); // opaque

        assert_ok!(Pallet::<Runtime>::accept_pending_and_transfer(
            RuntimeOrigin::signed(BOB),
            ASSET,
            CHARLIE,
            accept_env,
            transfer_proof
        ));

        // After accept: BOB avail set to [3;32]; after transfer: from_new_available overwrites to [1;32]
        assert_eq!(
            AvailableBalanceCommit::<Runtime>::get(ASSET, BOB).unwrap(),
            [1u8; 32]
        );
        // CHARLIE pending updated to [2;32] and a UTXO with the transferred “amount”
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, CHARLIE).unwrap(),
            [2u8; 32]
        );
        assert_eq!(
            PendingDeposits::<Runtime>::get((CHARLIE, ASSET, 0)).unwrap(),
            // transfer_encrypted returns the same ciphertext it was passed; here it's the
            // “claimed” marker of 64 zeroes coming from claim_encrypted()
            [0u8; 64]
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(CHARLIE, ASSET), 1);

        match last_event() {
            RuntimeEvent::Zkhe(pallet::Event::PendingAcceptedAndTransferred {
                asset,
                from,
                to,
                encrypted_amount,
            }) => {
                assert_eq!(asset, ASSET);
                assert_eq!(from, BOB);
                assert_eq!(to, CHARLIE);
                assert_eq!(encrypted_amount, [0u8; 64]);
            }
            e => panic!("unexpected event: {e:?}"),
        }
    });
}

#[test]
fn mint_encrypted_updates_pending_total_and_records_utxo() {
    new_test_ext().execute_with(|| {
        set_pk(BOB);

        let proof = proof(&[]);
        let minted =
            <Pallet<Runtime> as ConfidentialBackend<_, _, _>>::mint_encrypted(ASSET, &BOB, proof)
                .expect("ok");

        assert_eq!(minted, [5u8; 64]);
        assert_eq!(
            PendingBalanceCommit::<Runtime>::get(ASSET, BOB).unwrap(),
            [10u8; 32]
        );
        assert_eq!(
            TotalSupplyCommit::<Runtime>::get(ASSET).unwrap(),
            [11u8; 32]
        );

        assert_eq!(
            PendingDeposits::<Runtime>::get((BOB, ASSET, 0)).unwrap(),
            [5u8; 64]
        );
        assert_eq!(NextPendingDepositId::<Runtime>::get(BOB, ASSET), 1);
    });
}

#[test]
fn burn_encrypted_updates_available_total_and_returns_amount() {
    new_test_ext().execute_with(|| {
        set_pk(ALICE);

        // Seed some pre-state (optional; verifier ignores)
        AvailableBalanceCommit::<Runtime>::insert(ASSET, ALICE, [9u8; 32]);
        TotalSupplyCommit::<Runtime>::insert(ASSET, [8u8; 32]);

        let amt = <Pallet<Runtime> as ConfidentialBackend<_, _, _>>::burn_encrypted(
            ASSET,
            &ALICE,
            ct(77),
            proof(&[4, 4, 4]),
        )
        .expect("ok");

        // Mock returns disclosed 42, and new commits [20;32], [21;32]
        assert_eq!(amt, 42u64);
        assert_eq!(
            AvailableBalanceCommit::<Runtime>::get(ASSET, ALICE).unwrap(),
            [20u8; 32]
        );
        assert_eq!(
            TotalSupplyCommit::<Runtime>::get(ASSET).unwrap(),
            [21u8; 32]
        );
    });
}

#[test]
fn errors_no_public_key_and_malformed_envelope() {
    new_test_ext().execute_with(|| {
        // No PK for ALICE -> transfer should fail with NoPublicKey
        set_pk(BOB); // only receiver has pk
        let err = Pallet::<Runtime>::transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            BOB,
            ct(1),
            proof(&[]),
        )
        .unwrap_err();
        assert_eq!(err, Error::<Runtime>::NoPublicKey.into());

        // accept_pending with malformed envelope: must be at least 2 bytes for count
        set_pk(ALICE);
        let bad = proof(&[1]); // too short
        let err = Pallet::<Runtime>::accept_pending(RuntimeOrigin::signed(ALICE), ASSET, bad)
            .unwrap_err();
        assert_eq!(err, Error::<Runtime>::MalformedEnvelope.into());
    });
}

#[test]
fn origin_checks_on_dispatchables() {
    new_test_ext().execute_with(|| {
        // Unsigned should be BadOrigin on dispatchables
        assert!(matches!(
            Pallet::<Runtime>::transfer(
                RuntimeOrigin::none(),
                ASSET,
                BOB,
                ct(9),
                proof(&[])
            ),
            Err(e) if e == BadOrigin.into()
        ));
        assert!(matches!(
            Pallet::<Runtime>::accept_pending(RuntimeOrigin::none(), ASSET, accept_input(&[], &[])),
            Err(e) if e == BadOrigin.into()
        ));
    });
}

// ===================== PROPERTY TESTS =====================

prop_compose! {
    /// Generate arbitrary account IDs (non-zero to avoid collisions with system accounts)
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
    /// Generate arbitrary ciphertext fill byte
    fn arb_ciphertext()(fill in any::<u8>()) -> EncryptedAmount {
        [fill; 64]
    }
}

prop_compose! {
    /// Generate valid accept input with arbitrary pending deposit IDs
    fn arb_accept_input(max_ids: usize)(
        ids in prop::collection::vec(0u64..10, 0..max_ids),
        rest in prop::collection::vec(any::<u8>(), 0..100)
    ) -> InputProof {
        accept_input(&ids, &rest)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Transfer between any two distinct accounts with PKs set should succeed
    /// and update storage consistently
    #[test]
    fn prop_transfer_succeeds_with_valid_pks(
        sender in arb_account(),
        receiver in arb_account().prop_filter("receiver != sender", |r| *r != 1),
        asset in arb_asset(),
        ct_val in any::<u8>()
    ) {
        // Ensure sender != receiver
        let receiver = if receiver == sender { receiver + 1 } else { receiver };

        new_test_ext().execute_with(|| {
            // Setup: both parties need PKs
            set_pk(sender);
            set_pk(receiver);

            let delta = ct(ct_val);
            let prf = proof(&[1, 2, 3]);

            // Execute transfer
            let result = Pallet::<Runtime>::transfer(
                RuntimeOrigin::signed(sender),
                asset,
                receiver,
                delta,
                prf
            );

            // Assert success
            prop_assert!(result.is_ok(), "Transfer should succeed: {:?}", result);

            // Verify storage updates (mock returns [1;32] for from, [2;32] for to)
            prop_assert_eq!(
                AvailableBalanceCommit::<Runtime>::get(asset, sender),
                Some([1u8; 32]),
                "Sender available commit should be updated"
            );
            prop_assert_eq!(
                PendingBalanceCommit::<Runtime>::get(asset, receiver),
                Some([2u8; 32]),
                "Receiver pending commit should be updated"
            );

            // UTXO should be recorded
            prop_assert_eq!(
                PendingDeposits::<Runtime>::get((receiver, asset, 0)),
                Some(delta),
                "UTXO should be recorded for receiver"
            );

            Ok(())
        })?;
    }

    /// Property: Transfer fails without sender PK
    #[test]
    fn prop_transfer_fails_without_sender_pk(
        sender in arb_account(),
        receiver in arb_account(),
        asset in arb_asset()
    ) {
        let receiver = if receiver == sender { receiver + 1 } else { receiver };

        new_test_ext().execute_with(|| {
            // Only set receiver PK, not sender
            set_pk(receiver);

            let result = Pallet::<Runtime>::transfer(
                RuntimeOrigin::signed(sender),
                asset,
                receiver,
                ct(1),
                proof(&[])
            );

            prop_assert!(result.is_err(), "Transfer should fail without sender PK");
            prop_assert_eq!(result.unwrap_err(), Error::<Runtime>::NoPublicKey.into());

            Ok(())
        })?;
    }

    /// Property: Multiple sequential transfers to the same receiver increment UTXO IDs
    #[test]
    fn prop_sequential_transfers_increment_utxo_ids(
        sender in arb_account(),
        receiver in arb_account(),
        asset in arb_asset(),
        num_transfers in 1usize..5
    ) {
        let receiver = if receiver == sender { receiver + 1 } else { receiver };

        new_test_ext().execute_with(|| {
            set_pk(sender);
            set_pk(receiver);

            for i in 0..num_transfers {
                let ct_val = i as u8;
                assert_ok!(Pallet::<Runtime>::transfer(
                    RuntimeOrigin::signed(sender),
                    asset,
                    receiver,
                    ct(ct_val),
                    proof(&[])
                ));

                // Verify UTXO ID increments
                prop_assert_eq!(
                    NextPendingDepositId::<Runtime>::get(receiver, asset),
                    (i + 1) as u64,
                    "UTXO ID should increment after each transfer"
                );

                // Verify UTXO exists at expected index
                prop_assert!(
                    PendingDeposits::<Runtime>::get((receiver, asset, i as u64)).is_some(),
                    "UTXO should exist at index {}", i
                );
            }

            Ok(())
        })?;
    }

    /// Property: accept_pending with malformed envelope (too short) always fails
    #[test]
    fn prop_accept_pending_rejects_malformed_envelope(
        who in arb_account(),
        asset in arb_asset(),
        short_data in prop::collection::vec(any::<u8>(), 0..2)
    ) {
        new_test_ext().execute_with(|| {
            set_pk(who);

            // Envelope must be at least 2 bytes for u16 count
            let bad_envelope = proof(&short_data);

            let result = Pallet::<Runtime>::accept_pending(
                RuntimeOrigin::signed(who),
                asset,
                bad_envelope
            );

            // Should fail with MalformedEnvelope if < 2 bytes
            if short_data.len() < 2 {
                prop_assert!(result.is_err(), "Should reject envelope < 2 bytes");
                prop_assert_eq!(result.unwrap_err(), Error::<Runtime>::MalformedEnvelope.into());
            }

            Ok(())
        })?;
    }

    /// Property: Mint creates valid pending state
    #[test]
    fn prop_mint_creates_valid_pending_state(
        recipient in arb_account(),
        asset in arb_asset()
    ) {
        new_test_ext().execute_with(|| {
            set_pk(recipient);

            let result = <Pallet<Runtime> as ConfidentialBackend<_, _, _>>::mint_encrypted(
                asset,
                &recipient,
                proof(&[])
            );

            prop_assert!(result.is_ok(), "Mint should succeed: {:?}", result);

            // Mock returns [5;64] as minted ciphertext
            prop_assert_eq!(result.unwrap(), [5u8; 64]);

            // Check storage updates
            prop_assert_eq!(
                PendingBalanceCommit::<Runtime>::get(asset, recipient),
                Some([10u8; 32]),
                "Pending commit should be set"
            );
            prop_assert_eq!(
                TotalSupplyCommit::<Runtime>::get(asset),
                Some([11u8; 32]),
                "Total supply commit should be set"
            );

            // UTXO should be recorded
            prop_assert!(
                PendingDeposits::<Runtime>::get((recipient, asset, 0)).is_some(),
                "UTXO should be recorded"
            );

            Ok(())
        })?;
    }

    /// Property: Burn with pre-existing state updates commits correctly
    #[test]
    fn prop_burn_updates_commits_correctly(
        burner in arb_account(),
        asset in arb_asset(),
        initial_avail in any::<u8>(),
        initial_total in any::<u8>()
    ) {
        new_test_ext().execute_with(|| {
            set_pk(burner);

            // Seed initial state
            AvailableBalanceCommit::<Runtime>::insert(asset, burner, [initial_avail; 32]);
            TotalSupplyCommit::<Runtime>::insert(asset, [initial_total; 32]);

            let result = <Pallet<Runtime> as ConfidentialBackend<_, _, _>>::burn_encrypted(
                asset,
                &burner,
                ct(77),
                proof(&[])
            );

            prop_assert!(result.is_ok(), "Burn should succeed: {:?}", result);

            // Mock returns disclosed amount 42
            prop_assert_eq!(result.unwrap(), 42u64);

            // Check storage updates (mock returns [20;32], [21;32])
            prop_assert_eq!(
                AvailableBalanceCommit::<Runtime>::get(asset, burner),
                Some([20u8; 32]),
                "Available commit should be updated after burn"
            );
            prop_assert_eq!(
                TotalSupplyCommit::<Runtime>::get(asset),
                Some([21u8; 32]),
                "Total supply commit should be updated after burn"
            );

            Ok(())
        })?;
    }
}
