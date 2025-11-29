//! Unit tests for the confidential assets EVM precompile.

use crate::mock::{
    precompiles, set_pk, ConfidentialAssetsAddress, ExtBuilder, PCall, Precompiles,
    PrecompilesValue, Runtime,
};
use precompile_utils::prelude::Address;
use precompile_utils::testing::*;
use sp_core::{H160, H256, U256};

#[allow(dead_code)]
fn precompiles_instance() -> Precompiles<Runtime> {
    PrecompilesValue::get()
}

/// Helper to convert test accounts to Address
fn addr<T: Into<H160>>(account: T) -> Address {
    Address(account.into())
}

// ============ Selector Tests ============

#[test]
fn selectors_are_correct() {
    // Verify function selectors match expected values
    // These selectors are computed from keccak256 of the Solidity function signature
    assert!(PCall::confidential_balance_of_selectors().len() > 0);
    assert!(PCall::confidential_total_supply_selectors().len() > 0);
    assert!(PCall::name_selectors().len() > 0);
    assert!(PCall::symbol_selectors().len() > 0);
    assert!(PCall::decimals_selectors().len() > 0);
    assert!(PCall::set_public_key_selectors().len() > 0);
    assert!(PCall::deposit_selectors().len() > 0);
    assert!(PCall::withdraw_selectors().len() > 0);
    assert!(PCall::confidential_transfer_selectors().len() > 0);
    assert!(PCall::confidential_claim_selectors().len() > 0);
}

#[test]
fn selectors_match_solidity_interface() {
    // This test verifies the precompile implements the exact selectors
    // defined in contracts/interfaces/IConfidentialAssets.sol
    //
    // The selectors are computed as: keccak256("functionName(arg1Type,arg2Type,...)")[:4]

    use precompile_utils::testing::compute_selector;

    // View functions
    assert_eq!(
        PCall::confidential_balance_of_selectors()[0],
        compute_selector("confidentialBalanceOf(uint128,address)"),
        "confidentialBalanceOf selector mismatch"
    );
    assert_eq!(
        PCall::confidential_total_supply_selectors()[0],
        compute_selector("confidentialTotalSupply(uint128)"),
        "confidentialTotalSupply selector mismatch"
    );
    assert_eq!(
        PCall::name_selectors()[0],
        compute_selector("name(uint128)"),
        "name selector mismatch"
    );
    assert_eq!(
        PCall::symbol_selectors()[0],
        compute_selector("symbol(uint128)"),
        "symbol selector mismatch"
    );
    assert_eq!(
        PCall::decimals_selectors()[0],
        compute_selector("decimals(uint128)"),
        "decimals selector mismatch"
    );

    // State-changing functions
    assert_eq!(
        PCall::set_public_key_selectors()[0],
        compute_selector("setPublicKey(bytes)"),
        "setPublicKey selector mismatch"
    );
    assert_eq!(
        PCall::deposit_selectors()[0],
        compute_selector("deposit(uint128,uint256,bytes)"),
        "deposit selector mismatch"
    );
    assert_eq!(
        PCall::withdraw_selectors()[0],
        compute_selector("withdraw(uint128,bytes,bytes)"),
        "withdraw selector mismatch"
    );
    assert_eq!(
        PCall::confidential_transfer_selectors()[0],
        compute_selector("confidentialTransfer(uint128,address,bytes,bytes)"),
        "confidentialTransfer selector mismatch"
    );
    assert_eq!(
        PCall::confidential_claim_selectors()[0],
        compute_selector("confidentialClaim(uint128,bytes)"),
        "confidentialClaim selector mismatch"
    );
}

#[test]
fn print_selectors_for_solidity_interface() {
    // This test prints the selectors for documentation purposes.
    // Run with: cargo test -p confidential-assets-evm-precompile print_selectors -- --nocapture

    use precompile_utils::testing::compute_selector;

    println!("\n=== Confidential Assets Precompile Selectors ===\n");

    let functions = [
        "confidentialBalanceOf(uint128,address)",
        "confidentialTotalSupply(uint128)",
        "name(uint128)",
        "symbol(uint128)",
        "decimals(uint128)",
        "setPublicKey(bytes)",
        "deposit(uint128,uint256,bytes)",
        "withdraw(uint128,bytes,bytes)",
        "confidentialTransfer(uint128,address,bytes,bytes)",
        "confidentialClaim(uint128,bytes)",
    ];

    for sig in functions {
        let selector = compute_selector(sig);
        println!("/// @custom:selector {:08x}", selector);
        println!("function {} external;", sig);
        println!();
    }
}

#[test]
fn precompile_matches_solidity_interface_file() {
    // This test uses check_precompile_implements_solidity_interfaces to verify
    // the precompile matches the Solidity interface file.
    //
    // It parses the .sol file, extracts @custom:selector annotations,
    // and verifies the precompile supports each selector.

    use precompile_utils::testing::check_precompile_implements_solidity_interfaces;

    // The path is relative to the crate root
    check_precompile_implements_solidity_interfaces(
        &["../../contracts/interfaces/IConfidentialAssets.sol"],
        PCall::supports_selector,
    );
}

// ============ View Function Tests ============

#[test]
fn test_confidential_balance_of_returns_zero_for_unknown_account() {
    ExtBuilder::default().build().execute_with(|| {
        // Query balance for an account with no balance set
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::confidential_balance_of {
                    asset: 1u128,
                    who: addr(Alice),
                },
            )
            .execute_returns(H256::zero());
    })
}

#[test]
fn test_confidential_total_supply_returns_zero_for_unknown_asset() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::confidential_total_supply { asset: 999u128 },
            )
            .execute_returns(H256::zero());
    })
}

#[test]
fn test_name_returns_empty_for_unregistered_asset() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::name { asset: 1u128 },
            )
            .execute_returns(precompile_utils::prelude::UnboundedBytes::from(
                Vec::<u8>::new(),
            ));
    })
}

#[test]
fn test_symbol_returns_empty_for_unregistered_asset() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::symbol { asset: 1u128 },
            )
            .execute_returns(precompile_utils::prelude::UnboundedBytes::from(
                Vec::<u8>::new(),
            ));
    })
}

#[test]
fn test_decimals_returns_zero_for_unregistered_asset() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::decimals { asset: 1u128 },
            )
            .execute_returns(0u8);
    })
}

// ============ State-Changing Function Tests ============

#[test]
fn test_set_public_key_succeeds() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            // Set a public key (64 bytes)
            let pubkey = vec![0xABu8; 64];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::set_public_key {
                        pubkey: pubkey.into(),
                    },
                )
                .execute_returns(());
        })
}

#[test]
fn test_set_public_key_rejects_oversized_key() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            // Try to set a key larger than MAX_PUBKEY_SIZE (64 bytes)
            let oversized_pubkey = vec![0xABu8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::set_public_key {
                        pubkey: oversized_pubkey.into(),
                    },
                )
                .execute_reverts(|output| {
                    // precompile_utils returns "pubkey: Value is too large for length"
                    output == b"pubkey: Value is too large for length"
                });
        })
}

#[test]
fn test_deposit_succeeds_with_valid_proof() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            // First set up a public key for Alice
            set_pk(Alice.into());

            // Deposit with a mock proof
            let proof_data = vec![0x01u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: 1u128,
                        amount: U256::from(1000u64),
                        proof: proof_data.into(),
                    },
                )
                .execute_returns(());
        })
}

#[test]
fn test_deposit_rejects_oversized_proof() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            set_pk(Alice.into());

            // Try with a proof larger than MAX_PROOF_SIZE (8192 bytes)
            let oversized_proof = vec![0x01u8; 9000];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: 1u128,
                        amount: U256::from(1000u64),
                        proof: oversized_proof.into(),
                    },
                )
                .execute_reverts(|output| {
                    // precompile_utils returns "proof: Value is too large for length"
                    output == b"proof: Value is too large for length"
                });
        })
}

#[test]
fn test_withdraw_succeeds_with_valid_inputs() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            set_pk(Alice.into());

            // First deposit some funds
            let deposit_proof = vec![0x01u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: 1u128,
                        amount: U256::from(1000u64),
                        proof: deposit_proof.into(),
                    },
                )
                .execute_returns(());

            // Then withdraw
            let encrypted_amount = vec![0x02u8; 64]; // 64 bytes exactly
            let withdraw_proof = vec![0x03u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::withdraw {
                        asset: 1u128,
                        encrypted_amount: encrypted_amount.into(),
                        proof: withdraw_proof.into(),
                    },
                )
                .execute_returns(());
        })
}

#[test]
fn test_withdraw_rejects_wrong_size_encrypted_amount() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            set_pk(Alice.into());

            // Try with wrong size encrypted amount (not 64 bytes)
            let wrong_size_amount = vec![0x02u8; 32]; // Should be 64 bytes
            let proof = vec![0x03u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::withdraw {
                        asset: 1u128,
                        encrypted_amount: wrong_size_amount.into(),
                        proof: proof.into(),
                    },
                )
                .execute_reverts(|output| output == b"encrypted amount must be 64 bytes");
        })
}

#[test]
fn test_confidential_transfer_succeeds() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000), (Bob.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            // Set public keys for both parties
            set_pk(Alice.into());
            set_pk(Bob.into());

            // Deposit funds for Alice first
            let deposit_proof = vec![0x01u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: 1u128,
                        amount: U256::from(1000u64),
                        proof: deposit_proof.into(),
                    },
                )
                .execute_returns(());

            // Transfer from Alice to Bob
            let encrypted_amount = vec![0x05u8; 64];
            let transfer_proof = vec![0x06u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::confidential_transfer {
                        asset: 1u128,
                        to: addr(Bob),
                        encrypted_amount: encrypted_amount.into(),
                        proof: transfer_proof.into(),
                    },
                )
                .execute_returns(());
        })
}

#[test]
fn test_confidential_claim_succeeds() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000), (Bob.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            set_pk(Alice.into());
            set_pk(Bob.into());

            // Deposit funds for Alice
            let deposit_proof = vec![0x01u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: 1u128,
                        amount: U256::from(1000u64),
                        proof: deposit_proof.into(),
                    },
                )
                .execute_returns(());

            // Transfer from Alice to Bob
            let encrypted_amount = vec![0x05u8; 64];
            let transfer_proof = vec![0x06u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::confidential_transfer {
                        asset: 1u128,
                        to: addr(Bob),
                        encrypted_amount: encrypted_amount.into(),
                        proof: transfer_proof.into(),
                    },
                )
                .execute_returns(());

            // Bob claims the pending transfer
            // The claim proof needs to encode the transfer IDs
            let mut claim_proof = Vec::new();
            // count: 1 transfer
            claim_proof.extend_from_slice(&1u16.to_le_bytes());
            // transfer_id: 0 (first transfer)
            claim_proof.extend_from_slice(&0u64.to_le_bytes());
            // rest of proof data
            claim_proof.extend_from_slice(&[0x07u8; 50]);

            precompiles()
                .prepare_test(
                    Bob,
                    ConfidentialAssetsAddress,
                    PCall::confidential_claim {
                        asset: 1u128,
                        proof: claim_proof.into(),
                    },
                )
                .execute_returns(());
        })
}

// ============ Edge Case Tests ============

#[test]
fn test_view_functions_work_in_static_context() {
    ExtBuilder::default().build().execute_with(|| {
        // View functions should work in static (read-only) context
        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::confidential_balance_of {
                    asset: 1u128,
                    who: addr(Alice),
                },
            )
            .with_static_call(true)
            .execute_returns(H256::zero());

        precompiles()
            .prepare_test(
                Alice,
                ConfidentialAssetsAddress,
                PCall::decimals { asset: 1u128 },
            )
            .with_static_call(true)
            .execute_returns(0u8);
    })
}

#[test]
fn test_state_changing_fails_in_static_context() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            let pubkey = vec![0xABu8; 64];
            // State-changing functions should fail in static context
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::set_public_key {
                        pubkey: pubkey.into(),
                    },
                )
                .with_static_call(true)
                .execute_reverts(|_| true); // Should revert with some error
        })
}

// ============ Modifier Tests ============

#[test]
fn modifiers() {
    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1000)])
        .build()
        .execute_with(|| {
            let mut tester =
                PrecompilesModifierTester::new(precompiles(), Alice, ConfidentialAssetsAddress);

            // View functions should have view modifier
            tester.test_view_modifier(PCall::confidential_balance_of_selectors());
            tester.test_view_modifier(PCall::confidential_total_supply_selectors());
            tester.test_view_modifier(PCall::name_selectors());
            tester.test_view_modifier(PCall::symbol_selectors());
            tester.test_view_modifier(PCall::decimals_selectors());

            // State-changing functions should have default (non-view, non-payable) modifier
            tester.test_default_modifier(PCall::set_public_key_selectors());
            tester.test_default_modifier(PCall::deposit_selectors());
            tester.test_default_modifier(PCall::withdraw_selectors());
            tester.test_default_modifier(PCall::confidential_transfer_selectors());
            tester.test_default_modifier(PCall::confidential_claim_selectors());
        });
}

// ============ ERC-7984 Compatibility Tests ============
//
// These tests verify that the precompile interface is compatible with the
// ERC7984ConfidentialToken wrapper contract. The wrapper translates ERC-7984
// interface calls to precompile calls.
//
// The wrapper contract (contracts/ERC7984ConfidentialToken.sol):
// - Binds to a specific asset ID at deployment
// - Translates single-asset ERC-7984 calls to multi-asset precompile calls
// - Maps: IERC7984.confidentialBalanceOf(address) -> precompile.confidentialBalanceOf(assetId, address)
// - Maps: IERC7984.confidentialTotalSupply() -> precompile.confidentialTotalSupply(assetId)
// - etc.

#[test]
fn erc7984_wrapper_uses_correct_precompile_selectors() {
    // The ERC7984ConfidentialToken wrapper calls the precompile at 0x800 using these selectors.
    // This test verifies the precompile implements the exact selectors the wrapper expects.
    //
    // From contracts/ERC7984ConfidentialToken.sol:
    //   PRECOMPILE.confidentialBalanceOf(assetId, account)  -> selector cd40095b
    //   PRECOMPILE.confidentialTotalSupply(assetId)         -> selector efa18641
    //   PRECOMPILE.name(assetId)                            -> selector c624440a
    //   PRECOMPILE.symbol(assetId)                          -> selector 117f1264
    //   PRECOMPILE.decimals(assetId)                        -> selector 09d2f9b4
    //   PRECOMPILE.setPublicKey(pubkey)                     -> selector a91d58b4
    //   PRECOMPILE.deposit(assetId, amount, proof)          -> selector 94679bd1
    //   PRECOMPILE.withdraw(assetId, encryptedAmount, proof) -> selector f1f9153b
    //   PRECOMPILE.confidentialTransfer(assetId, to, encryptedAmount, proof) -> selector f49a002f
    //   PRECOMPILE.confidentialClaim(assetId, proof)        -> selector 12cb9d88

    use precompile_utils::testing::compute_selector;

    // Verify each selector the wrapper expects is implemented by the precompile
    let wrapper_expected_selectors = [
        ("confidentialBalanceOf(uint128,address)", 0xcd40095bu32),
        ("confidentialTotalSupply(uint128)", 0xefa18641u32),
        ("name(uint128)", 0xc624440au32),
        ("symbol(uint128)", 0x117f1264u32),
        ("decimals(uint128)", 0x09d2f9b4u32),
        ("setPublicKey(bytes)", 0xa91d58b4u32),
        ("deposit(uint128,uint256,bytes)", 0x94679bd1u32),
        ("withdraw(uint128,bytes,bytes)", 0xf1f9153bu32),
        (
            "confidentialTransfer(uint128,address,bytes,bytes)",
            0xf49a002fu32,
        ),
        ("confidentialClaim(uint128,bytes)", 0x12cb9d88u32),
    ];

    for (signature, expected_selector) in wrapper_expected_selectors {
        let computed = compute_selector(signature);
        assert_eq!(
            computed, expected_selector,
            "Selector mismatch for {}: expected {:08x}, got {:08x}",
            signature, expected_selector, computed
        );

        // Also verify the precompile supports this selector
        assert!(
            PCall::supports_selector(computed),
            "Precompile does not support selector {:08x} for {}. \
             The ERC7984ConfidentialToken wrapper will fail to call this function.",
            computed,
            signature
        );
    }
}

#[test]
fn erc7984_interface_selectors_documented() {
    // This test documents the ERC-7984 interface selectors for reference.
    // These are the selectors that ERC-7984 consumers use to call the wrapper.
    //
    // From EIP-7984 (https://eips.ethereum.org/EIPS/eip-7984):
    //
    // The wrapper contract implements IERC7984 and translates these calls
    // to the multi-asset precompile interface.

    use precompile_utils::testing::compute_selector;

    // ERC-7984 standard function signatures and their computed selectors
    let erc7984_functions = [
        // View functions (standard ERC-7984)
        "name()",
        "symbol()",
        "decimals()",
        "contractURI()",
        "confidentialTotalSupply()",
        "confidentialBalanceOf(address)",
        "isOperator(address,address)",
        // State-changing functions (standard ERC-7984)
        "setOperator(address,uint48)",
        "confidentialTransfer(address,bytes32)",
        "confidentialTransfer(address,bytes32,bytes)",
        "confidentialTransferFrom(address,address,bytes32)",
        "confidentialTransferFrom(address,address,bytes32,bytes)",
        "confidentialTransferAndCall(address,bytes32,bytes)",
        "confidentialTransferAndCall(address,bytes32,bytes,bytes)",
        "confidentialTransferFromAndCall(address,address,bytes32,bytes)",
        "confidentialTransferFromAndCall(address,address,bytes32,bytes,bytes)",
    ];

    // Print selectors for documentation (visible with --nocapture)
    println!("\n=== ERC-7984 Interface Selectors ===");
    println!("(These are what consumers call on the wrapper)\n");
    for sig in erc7984_functions {
        println!("{:08x}: {}", compute_selector(sig), sig);
    }
    println!();

    // Verify the well-known ERC-20 compatible selectors match
    // (name, symbol, decimals are shared with ERC-20)
    assert_eq!(compute_selector("name()"), 0x06fdde03);
    assert_eq!(compute_selector("symbol()"), 0x95d89b41);
    assert_eq!(compute_selector("decimals()"), 0x313ce567);
}

#[test]
fn erc7984_wrapper_correctly_maps_interface() {
    // This test documents the mapping between ERC-7984 and the precompile.
    // The wrapper contract translates calls as follows:
    //
    // ERC-7984 Interface          ->  Precompile Interface
    // -------------------------       ----------------------
    // name()                      ->  name(assetId)
    // symbol()                    ->  symbol(assetId)
    // decimals()                  ->  decimals(assetId)
    // confidentialTotalSupply()   ->  confidentialTotalSupply(assetId)
    // confidentialBalanceOf(addr) ->  confidentialBalanceOf(assetId, addr)
    // isOperator(h,s)             ->  [managed by wrapper, no precompile call]
    // setOperator(op, until)      ->  [managed by wrapper, no precompile call]
    // confidentialTransfer(...)   ->  confidentialTransfer(assetId, to, encryptedAmt, proof)
    // confidentialTransferFrom    ->  [checks operator] + confidentialTransfer(...)
    //
    // Additional wrapper functions:
    // setPublicKey(pubkey)        ->  setPublicKey(pubkey)
    // deposit(amount, proof)      ->  deposit(assetId, amount, proof)
    // withdraw(encAmt, proof)     ->  withdraw(assetId, encAmt, proof)
    // claim(proof)                ->  confidentialClaim(assetId, proof)

    use precompile_utils::testing::compute_selector;

    // The precompile adds an assetId parameter to single-asset ERC-7984 functions.
    // Verify this mapping is correct:

    // Single-asset (ERC-7984) vs Multi-asset (Precompile)
    let mappings = [
        (
            "confidentialBalanceOf(address)",
            "confidentialBalanceOf(uint128,address)",
        ),
        (
            "confidentialTotalSupply()",
            "confidentialTotalSupply(uint128)",
        ),
        ("name()", "name(uint128)"),
        ("symbol()", "symbol(uint128)"),
        ("decimals()", "decimals(uint128)"),
    ];

    for (erc7984_sig, precompile_sig) in mappings {
        let erc7984_sel = compute_selector(erc7984_sig);
        let precompile_sel = compute_selector(precompile_sig);

        // They should be different (precompile adds assetId)
        assert_ne!(
            erc7984_sel, precompile_sel,
            "ERC-7984 {} and precompile {} should have different selectors since precompile adds assetId",
            erc7984_sig, precompile_sig
        );

        // But precompile should support its selector
        assert!(
            PCall::supports_selector(precompile_sel),
            "Precompile must support {} (selector {:08x}) for ERC-7984 wrapper compatibility",
            precompile_sig,
            precompile_sel
        );
    }
}

#[test]
fn erc7984_wrapper_integration_scenario() {
    // This test simulates the full flow that an ERC-7984 consumer would use
    // when interacting with the wrapper contract, verifying the precompile
    // supports all required operations.
    //
    // The wrapper contract (ERC7984ConfidentialToken.sol) would call these
    // precompile functions in response to ERC-7984 interface calls.

    ExtBuilder::default()
        .with_balances(vec![(Alice.into(), 1_000_000), (Bob.into(), 1_000_000)])
        .build()
        .execute_with(|| {
            // Step 1: Users set up public keys (via wrapper's setPublicKey)
            // ERC-7984 consumer calls: wrapper.setPublicKey(pubkey)
            // Wrapper calls: PRECOMPILE.setPublicKey(pubkey)
            let alice_pk = vec![0xAAu8; 64];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::set_public_key {
                        pubkey: alice_pk.into(),
                    },
                )
                .execute_returns(());

            let bob_pk = vec![0xBBu8; 64];
            precompiles()
                .prepare_test(
                    Bob,
                    ConfidentialAssetsAddress,
                    PCall::set_public_key {
                        pubkey: bob_pk.into(),
                    },
                )
                .execute_returns(());

            // Step 2: Alice deposits tokens (via wrapper's deposit)
            // ERC-7984 consumer calls: wrapper.deposit(amount, proof)
            // Wrapper calls: PRECOMPILE.deposit(assetId, amount, proof)
            let asset_id = 1u128; // Wrapper binds to this at deployment
            let deposit_proof = vec![0x01u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::deposit {
                        asset: asset_id,
                        amount: U256::from(10000u64),
                        proof: deposit_proof.into(),
                    },
                )
                .execute_returns(());

            // Step 3: Query balance (via wrapper's confidentialBalanceOf)
            // ERC-7984 consumer calls: wrapper.confidentialBalanceOf(account)
            // Wrapper calls: PRECOMPILE.confidentialBalanceOf(assetId, account)
            // Note: We just verify the call succeeds, actual value depends on mock
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::confidential_balance_of {
                        asset: asset_id,
                        who: addr(Alice),
                    },
                )
                .execute_some(); // Just verify it executes without error

            // Step 4: Alice transfers to Bob (via wrapper's confidentialTransfer)
            // ERC-7984 consumer calls: wrapper.confidentialTransfer(to, amountCommitment, data)
            // where data = abi.encode(encryptedAmount, proof)
            // Wrapper calls: PRECOMPILE.confidentialTransfer(assetId, to, encryptedAmount, proof)
            let encrypted_amount = vec![0x05u8; 64];
            let transfer_proof = vec![0x06u8; 100];
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::confidential_transfer {
                        asset: asset_id,
                        to: addr(Bob),
                        encrypted_amount: encrypted_amount.into(),
                        proof: transfer_proof.into(),
                    },
                )
                .execute_returns(());

            // Step 5: Bob claims (via wrapper's claim)
            // ERC-7984 consumer calls: wrapper.claim(proof)
            // Wrapper calls: PRECOMPILE.confidentialClaim(assetId, proof)
            let mut claim_proof = Vec::new();
            claim_proof.extend_from_slice(&1u16.to_le_bytes()); // count
            claim_proof.extend_from_slice(&0u64.to_le_bytes()); // transfer_id
            claim_proof.extend_from_slice(&[0x07u8; 50]); // proof data

            precompiles()
                .prepare_test(
                    Bob,
                    ConfidentialAssetsAddress,
                    PCall::confidential_claim {
                        asset: asset_id,
                        proof: claim_proof.into(),
                    },
                )
                .execute_returns(());

            // Step 6: Query total supply (via wrapper's confidentialTotalSupply)
            // ERC-7984 consumer calls: wrapper.confidentialTotalSupply()
            // Wrapper calls: PRECOMPILE.confidentialTotalSupply(assetId)
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::confidential_total_supply { asset: asset_id },
                )
                .execute_some(); // Just verify it executes

            // Step 7: Query metadata
            // ERC-7984 consumer calls: wrapper.name(), wrapper.symbol(), wrapper.decimals()
            // Wrapper either returns cached values or calls precompile
            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::name { asset: asset_id },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::symbol { asset: asset_id },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    Alice,
                    ConfidentialAssetsAddress,
                    PCall::decimals { asset: asset_id },
                )
                .execute_returns(0u8);
        })
}
