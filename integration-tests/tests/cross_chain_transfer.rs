//! Integration tests for cross-chain confidential transfers
//!
//! These tests verify confidential transfers between parachains using HRMP.
//! Requires the `zombienet` feature flag and Rust >= 1.88.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p integration-tests --features zombienet cross_chain -- --nocapture
//! ```

// ==============================================================================
// Tests that require zombienet (cross-chain needs live network)
// ==============================================================================

#[cfg(feature = "zombienet")]
mod zombienet_tests {
    use super::*;
    use integration_tests::network::{dual_parachain_config, wait_for_blocks};
    use integration_tests::{compress_pk, test_accounts};
    use zombienet_sdk::NetworkConfigExt;

    /// Test: Basic cross-chain confidential transfer
    #[tokio::test]
    #[ignore = "requires zombienet binaries and built node"]
    async fn test_cross_chain_transfer() -> Result<()> {
        info!("Starting cross-chain confidential transfer test");

        let config = dual_parachain_config()?;
        let network = config.spawn_native().await?;

        let para_a_node = network.get_node("collator-a")?;
        let para_b_node = network.get_node("collator-b")?;

        info!("ParaA URL: {}", para_a_node.ws_uri());
        info!("ParaB URL: {}", para_b_node.ws_uri());

        // Wait for HRMP channels to open
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        let (_alice_sk, alice_pk) = test_accounts::alice();
        let (_bob_sk, bob_pk) = test_accounts::bob();

        info!("Alice (ParaA): {:?}", &compress_pk(&alice_pk)[..8]);
        info!("Bob (ParaB): {:?}", &compress_pk(&bob_pk)[..8]);

        // TODO: Full cross-chain transfer via subxt

        drop(network);
        Ok(())
    }

    /// Test: Cross-chain timeout and refund
    #[tokio::test]
    #[ignore = "requires zombienet binaries and built node"]
    async fn test_cross_chain_timeout_refund() -> Result<()> {
        info!("Starting cross-chain timeout refund test");

        let config = dual_parachain_config()?;
        let network = config.spawn_native().await?;

        // TODO: Test timeout/refund flow

        drop(network);
        Ok(())
    }

    /// Test: Bidirectional cross-chain transfers
    #[tokio::test]
    #[ignore = "requires zombienet binaries and built node"]
    async fn test_bidirectional_transfers() -> Result<()> {
        info!("Starting bidirectional cross-chain transfer test");

        let config = dual_parachain_config()?;
        let network = config.spawn_native().await?;

        // TODO: Transfers in both directions simultaneously

        drop(network);
        Ok(())
    }

    /// Test: Cross-chain stress test
    #[tokio::test]
    #[ignore = "requires zombienet binaries - long running"]
    async fn test_cross_chain_stress() -> Result<()> {
        use integration_tests::helpers::StressTestMetrics;
        use std::time::Instant;

        info!("Starting cross-chain stress test");

        let config = dual_parachain_config()?;
        let network = config.spawn_native().await?;

        let mut metrics = StressTestMetrics::default();
        let start = Instant::now();

        // TODO: Measure cross-chain TPS

        metrics.total_time_ms = start.elapsed().as_millis() as u64;
        metrics.print_summary();

        drop(network);
        Ok(())
    }
}

// ==============================================================================
// Placeholder test for when zombienet is not enabled
// ==============================================================================

#[cfg(not(feature = "zombienet"))]
#[test]
fn test_cross_chain_requires_zombienet_feature() {
    println!("Cross-chain tests require the 'zombienet' feature flag");
    println!("Run with: cargo test -p integration-tests --features zombienet");
}
