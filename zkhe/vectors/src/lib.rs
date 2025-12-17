//! Deterministic ZK proof vectors for benchmarking and testing.
//!
//! This crate provides pre-generated cryptographic proofs that pass
//! the on-chain ZK verifier, enabling accurate weight benchmarking.
#![cfg_attr(not(feature = "std"), no_std)]

mod generated;

pub use generated::*;
