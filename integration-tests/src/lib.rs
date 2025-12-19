//! Zombienet-SDK Integration Tests for Confidential Assets
//!
//! This crate provides integration tests that spawn real parachain networks
//! using zombienet-sdk and test confidential transfer functionality end-to-end.
//!
//! ## Overview
//!
//! zombienet-sdk is a Rust library for programmatically spawning and testing
//! Polkadot/Substrate networks. Unlike the CLI-based zombienet, it allows
//! writing tests in pure Rust with full control over the network lifecycle.
//!
//! ## Test Categories
//!
//! 1. **Single-chain tests**: Confidential transfers within a single parachain
//! 2. **Cross-chain tests**: Confidential transfers between parachains via HRMP
//! 3. **Stress tests**: Block filling and TPS measurement in live network
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all integration tests (requires zombienet binaries)
//! cargo test -p integration-tests
//!
//! # Run with logging
//! RUST_LOG=info cargo test -p integration-tests -- --nocapture
//!
//! # Run specific test
//! cargo test -p integration-tests confidential_transfer
//! ```
//!
//! ## Prerequisites
//!
//! 1. Built node binary: `cargo build --release -p confidential-asset-hub-node`
//! 2. Polkadot binary for relay chain (download or build from polkadot-sdk)
//!
//! ## Network Topology
//!
//! The tests spawn:
//! - 1 Relay chain with 2 validators
//! - 2 Parachains (ParaA: 1000, ParaB: 2000) with 1 collator each
//! - HRMP channels between parachains for cross-chain messaging

pub mod helpers;
pub mod network;

#[allow(unused_imports)]
use anyhow::Result;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand::RngCore;
use rand::rngs::OsRng;

/// Generate a random ElGamal keypair for testing
pub fn generate_test_keypair() -> (Scalar, RistrettoPoint) {
    let mut rng = OsRng;
    let sk = Scalar::from(rng.next_u64());
    let pk = sk * RISTRETTO_BASEPOINT_POINT;
    (sk, pk)
}

/// Compress a public key to 32 bytes
pub fn compress_pk(pk: &RistrettoPoint) -> [u8; 32] {
    pk.compress().to_bytes()
}

/// Test accounts with known keys for reproducible testing
pub mod test_accounts {
    use super::*;

    /// Alice's test keypair (deterministic for reproducibility)
    pub fn alice() -> (Scalar, RistrettoPoint) {
        let sk = Scalar::from(1u64);
        let pk = sk * RISTRETTO_BASEPOINT_POINT;
        (sk, pk)
    }

    /// Bob's test keypair
    pub fn bob() -> (Scalar, RistrettoPoint) {
        let sk = Scalar::from(2u64);
        let pk = sk * RISTRETTO_BASEPOINT_POINT;
        (sk, pk)
    }

    /// Charlie's test keypair
    pub fn charlie() -> (Scalar, RistrettoPoint) {
        let sk = Scalar::from(3u64);
        let pk = sk * RISTRETTO_BASEPOINT_POINT;
        (sk, pk)
    }
}
