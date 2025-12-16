//! Integration tests for confidential transfers on a single parachain
//!
//! These tests use zombienet-sdk to spawn a real network and execute
//! confidential transfers end-to-end.
//!
//! ## Running
//!
//! ```bash
//! # Build the node first
//! cargo build --release -p confidential-asset-hub-node
//!
//! # Run tests (requires zombienet feature and Rust >= 1.88)
//! RUST_LOG=info cargo test -p integration-tests --features zombienet -- --nocapture
//! ```

use anyhow::Result;
use integration_tests::{compress_pk, test_accounts};
use tracing::info;

// ==============================================================================
// Tests that work without zombienet (proof generation only)
// ==============================================================================

/// Test: Transfer with insufficient balance fails at proof generation
#[tokio::test]
async fn test_insufficient_balance_transfer() -> Result<()> {
    use integration_tests::helpers::{ConfidentialBalance, generate_test_transfer};

    info!("Starting insufficient balance test");

    let (_alice_sk, alice_pk) = test_accounts::alice();
    let (_bob_sk, bob_pk) = test_accounts::bob();

    // Alice only has 100 tokens
    let alice_balance =
        ConfidentialBalance::from_opening(100, curve25519_dalek::scalar::Scalar::from(42u64));
    let bob_pending = ConfidentialBalance::zero();

    // Try to transfer 1000 tokens (should fail in proof generation)
    let result = generate_test_transfer(
        &alice_pk,
        &bob_pk,
        &alice_balance,
        &bob_pending,
        1000, // More than available!
        1,
    );

    assert!(
        result.is_err(),
        "Transfer with insufficient balance should fail"
    );

    info!("Insufficient balance correctly rejected");
    Ok(())
}

/// Test: Valid transfer proof generation
#[tokio::test]
async fn test_valid_transfer_proof_generation() -> Result<()> {
    use integration_tests::helpers::{ConfidentialBalance, generate_test_transfer};

    info!("Starting valid transfer proof generation test");

    let (_alice_sk, alice_pk) = test_accounts::alice();
    let (_bob_sk, bob_pk) = test_accounts::bob();

    // Alice has 10000 tokens
    let alice_balance =
        ConfidentialBalance::from_opening(10000, curve25519_dalek::scalar::Scalar::from(42u64));
    let bob_pending = ConfidentialBalance::zero();

    // Transfer 1000 tokens
    let result = generate_test_transfer(&alice_pk, &bob_pk, &alice_balance, &bob_pending, 1000, 1);

    assert!(result.is_ok(), "Valid transfer should succeed");

    let proof = result.unwrap();
    assert_eq!(
        proof.encrypted_amount.len(),
        64,
        "Encrypted amount should be 64 bytes"
    );
    assert!(
        !proof.proof_bundle.is_empty(),
        "Proof bundle should not be empty"
    );

    info!("Valid transfer proof generated successfully");
    Ok(())
}

// ==============================================================================
// Tests that require zombienet (full network tests)
// ==============================================================================

#[cfg(feature = "zombienet")]
mod zombienet_tests {
    use super::*;
    use integration_tests::network::{single_parachain_config, wait_for_blocks};
    use zombienet_sdk::NetworkConfigExt;

    /// Test: Basic confidential transfer flow with live network
    #[tokio::test]
    #[ignore = "requires zombienet binaries and built node"]
    async fn test_basic_confidential_transfer() -> Result<()> {
        info!("Starting basic confidential transfer test");

        let config = single_parachain_config()?;
        let network = config.spawn_native().await?;

        let para_node = network.get_node("collator-a")?;
        let ws_url = para_node.ws_uri();
        info!("Parachain node URL: {}", ws_url);

        wait_for_blocks(&ws_url, 3).await?;

        let (_alice_sk, alice_pk) = test_accounts::alice();
        let (_bob_sk, bob_pk) = test_accounts::bob();

        info!("Alice PK: {:?}", &compress_pk(&alice_pk)[..8]);
        info!("Bob PK: {:?}", &compress_pk(&bob_pk)[..8]);

        // TODO: Full chain interaction via subxt

        drop(network);
        Ok(())
    }

    /// Test: Sequential transfers
    #[tokio::test]
    #[ignore = "requires zombienet binaries and built node"]
    async fn test_sequential_transfers() -> Result<()> {
        use integration_tests::helpers::{ConfidentialBalance, generate_test_transfer};

        info!("Starting sequential transfers test");

        let config = single_parachain_config()?;
        let network = config.spawn_native().await?;

        let (_alice_sk, alice_pk) = test_accounts::alice();
        let (_bob_sk, bob_pk) = test_accounts::bob();

        let alice_balance =
            ConfidentialBalance::from_opening(10000, curve25519_dalek::scalar::Scalar::from(42u64));
        let bob_pending = ConfidentialBalance::zero();

        let transfer1 =
            generate_test_transfer(&alice_pk, &bob_pk, &alice_balance, &bob_pending, 1000, 1)?;

        info!(
            "Transfer 1 proof generated: {} bytes",
            transfer1.proof_bundle.len()
        );

        drop(network);
        Ok(())
    }

    /// Test: Block filling stress test
    #[tokio::test]
    #[ignore = "requires zombienet binaries - long running"]
    async fn test_block_filling_stress() -> Result<()> {
        use integration_tests::helpers::StressTestMetrics;
        use integration_tests::network::stress_test_config;
        use std::time::Instant;

        info!("Starting block filling stress test");

        let config = stress_test_config(2)?;
        let network = config.spawn_native().await?;

        let mut metrics = StressTestMetrics::default();
        let start = Instant::now();

        // TODO: Actual stress test implementation

        metrics.total_time_ms = start.elapsed().as_millis() as u64;
        metrics.print_summary();

        drop(network);
        Ok(())
    }
}
