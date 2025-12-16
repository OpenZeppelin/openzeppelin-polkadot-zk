//! Network configuration and spawning utilities for zombienet-sdk
//!
//! This module provides helpers for configuring and spawning test networks
//! with confidential assets enabled.
//!
//! Note: Full functionality requires the `zombienet` feature flag.

#[allow(unused_imports)]
use anyhow::Result;

#[cfg(feature = "zombienet")]
use zombienet_sdk::{NetworkConfig, NetworkConfigBuilder, OrchestratorError};

/// Default paths for node binaries
#[cfg(feature = "zombienet")]
pub mod binaries {
    use std::path::PathBuf;

    /// Path to the relay chain binary (polkadot)
    pub fn relay_chain() -> PathBuf {
        std::env::var("RELAY_CHAIN_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./target/release/polkadot"))
    }

    /// Path to the parachain collator binary
    pub fn parachain() -> PathBuf {
        std::env::var("PARACHAIN_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./target/release/confidential-asset-hub-node"))
    }
}

/// Network configuration for single parachain tests
#[cfg(feature = "zombienet")]
pub fn single_parachain_config() -> Result<NetworkConfig, OrchestratorError> {
    NetworkConfigBuilder::new()
        .with_relaychain(|relay| {
            relay
                .with_chain("rococo-local")
                .with_default_command(binaries::relay_chain().to_string_lossy())
                .with_node(|node| node.with_name("alice").validator(true))
                .with_node(|node| node.with_name("bob").validator(true))
        })
        .with_parachain(|para| {
            para.with_id(1000)
                .with_default_command(binaries::parachain().to_string_lossy())
                .with_collator(|col| col.with_name("collator-a"))
        })
        .build()
}

/// Network configuration for cross-chain tests (two parachains)
#[cfg(feature = "zombienet")]
pub fn dual_parachain_config() -> Result<NetworkConfig, OrchestratorError> {
    NetworkConfigBuilder::new()
        .with_relaychain(|relay| {
            relay
                .with_chain("rococo-local")
                .with_default_command(binaries::relay_chain().to_string_lossy())
                .with_node(|node| node.with_name("alice").validator(true))
                .with_node(|node| node.with_name("bob").validator(true))
        })
        .with_parachain(|para| {
            para.with_id(1000)
                .with_default_command(binaries::parachain().to_string_lossy())
                .with_collator(|col| col.with_name("collator-a"))
        })
        .with_parachain(|para| {
            para.with_id(2000)
                .with_default_command(binaries::parachain().to_string_lossy())
                .with_collator(|col| col.with_name("collator-b"))
        })
        // HRMP channel from 1000 -> 2000
        .with_hrmp_channel(|channel| {
            channel
                .with_sender(1000)
                .with_recipient(2000)
                .with_max_capacity(8)
                .with_max_message_size(1048576)
        })
        // HRMP channel from 2000 -> 1000
        .with_hrmp_channel(|channel| {
            channel
                .with_sender(2000)
                .with_recipient(1000)
                .with_max_capacity(8)
                .with_max_message_size(1048576)
        })
        .build()
}

/// Network configuration for stress testing
#[cfg(feature = "zombienet")]
pub fn stress_test_config(num_collators: usize) -> Result<NetworkConfig, OrchestratorError> {
    let mut builder = NetworkConfigBuilder::new().with_relaychain(|relay| {
        relay
            .with_chain("rococo-local")
            .with_default_command(binaries::relay_chain().to_string_lossy())
            .with_node(|node| node.with_name("alice").validator(true))
            .with_node(|node| node.with_name("bob").validator(true))
            .with_node(|node| node.with_name("charlie").validator(true))
            .with_node(|node| node.with_name("dave").validator(true))
    });

    // Add parachain with multiple collators
    builder = builder.with_parachain(|para| {
        let mut para_builder = para
            .with_id(1000)
            .with_default_command(binaries::parachain().to_string_lossy());

        for i in 0..num_collators {
            para_builder =
                para_builder.with_collator(|col| col.with_name(format!("collator-{}", i)));
        }
        para_builder
    });

    builder.build()
}

/// Helper to wait for parachain blocks
pub async fn wait_for_blocks(ws_url: &str, num_blocks: u32) -> Result<()> {
    // This would use subxt to connect and wait for blocks
    // Simplified placeholder for now
    tracing::info!("Waiting for {} blocks on {}", num_blocks, ws_url);
    tokio::time::sleep(tokio::time::Duration::from_secs(num_blocks as u64 * 12)).await;
    Ok(())
}
