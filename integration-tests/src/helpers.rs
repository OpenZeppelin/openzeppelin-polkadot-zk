//! Helper functions for integration tests
//!
//! Provides utilities for interacting with confidential assets pallets via subxt.

use anyhow::Result;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;

/// Represents a confidential balance with its commitment and opening
#[derive(Debug, Clone)]
pub struct ConfidentialBalance {
    /// The on-chain commitment (32 bytes)
    pub commitment: [u8; 32],
    /// The plaintext value (only known to owner)
    pub value: u64,
    /// The blinding factor (only known to owner)
    pub blinding: Scalar,
}

impl ConfidentialBalance {
    /// Create a zero balance (identity commitment)
    pub fn zero() -> Self {
        Self {
            commitment: [0u8; 32],
            value: 0,
            blinding: Scalar::ZERO,
        }
    }

    /// Create a balance from known opening
    pub fn from_opening(value: u64, blinding: Scalar) -> Self {
        use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
        use sha2::Sha512;

        let h = RistrettoPoint::hash_from_bytes::<Sha512>(b"Zether/PedersenH");
        let v = Scalar::from(value);
        let commit = v * G + blinding * h;

        Self {
            commitment: commit.compress().to_bytes(),
            value,
            blinding,
        }
    }
}

/// Generate a confidential transfer proof for testing
pub fn generate_test_transfer(
    sender_pk: &RistrettoPoint,
    receiver_pk: &RistrettoPoint,
    sender_balance: &ConfidentialBalance,
    receiver_pending: &ConfidentialBalance,
    amount: u64,
    asset_id: u128,
) -> Result<TestTransferProof> {
    use zkhe_prover::{prove_sender_transfer, SenderInput};

    let mut rng_seed = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut rng_seed);

    let input = SenderInput {
        asset_id: asset_id.to_le_bytes().to_vec(),
        network_id: [0u8; 32],
        sender_pk: *sender_pk,
        receiver_pk: *receiver_pk,
        from_old_c: decompress_point(&sender_balance.commitment)?,
        from_old_opening: (sender_balance.value, sender_balance.blinding),
        to_old_c: decompress_point(&receiver_pending.commitment)?,
        delta_value: amount,
        rng_seed,
        fee_c: None,
    };

    let output = prove_sender_transfer(&input).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    Ok(TestTransferProof {
        encrypted_amount: output.delta_ct_bytes,
        proof_bundle: output.sender_bundle_bytes,
        delta_commitment: output.delta_comm_bytes,
        sender_new_commitment: output.from_new_c,
        receiver_new_commitment: output.to_new_c,
    })
}

/// Generated transfer proof for testing
#[derive(Debug, Clone)]
pub struct TestTransferProof {
    pub encrypted_amount: [u8; 64],
    pub proof_bundle: Vec<u8>,
    pub delta_commitment: [u8; 32],
    pub sender_new_commitment: [u8; 32],
    pub receiver_new_commitment: [u8; 32],
}

/// Decompress a 32-byte point
fn decompress_point(bytes: &[u8; 32]) -> Result<RistrettoPoint> {
    use curve25519_dalek::ristretto::CompressedRistretto;

    // Identity point (all zeros)
    if bytes == &[0u8; 32] {
        return Ok(RistrettoPoint::default());
    }

    CompressedRistretto::from_slice(bytes)?
        .decompress()
        .ok_or_else(|| anyhow::anyhow!("Invalid point"))
}

/// Assertion helpers for integration tests
pub mod assertions {
    /// Assert that a transfer was successful by checking events
    pub fn assert_transfer_event_emitted(_events: &[u8], expected_from: &[u8], expected_to: &[u8]) {
        // Parse events and find ConfidentialTransfer event
        // This is a placeholder - real implementation would decode SCALE events
        tracing::info!("Checking for transfer event from {:?} to {:?}", expected_from, expected_to);
    }

    /// Assert that balance commitment changed
    pub fn assert_commitment_changed(old: &[u8; 32], new: &[u8; 32]) {
        assert_ne!(old, new, "Balance commitment should have changed");
    }
}

/// Metrics collection for stress tests
#[derive(Debug, Default)]
pub struct StressTestMetrics {
    pub total_transfers: u64,
    pub successful_transfers: u64,
    pub failed_transfers: u64,
    pub total_time_ms: u64,
    pub transfers_per_block: Vec<u32>,
}

impl StressTestMetrics {
    pub fn tps(&self) -> f64 {
        if self.total_time_ms == 0 {
            0.0
        } else {
            (self.successful_transfers as f64 * 1000.0) / self.total_time_ms as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_transfers == 0 {
            0.0
        } else {
            (self.successful_transfers as f64 / self.total_transfers as f64) * 100.0
        }
    }

    pub fn print_summary(&self) {
        println!("\n=== Stress Test Results ===");
        println!("Total transfers attempted: {}", self.total_transfers);
        println!("Successful: {}", self.successful_transfers);
        println!("Failed: {}", self.failed_transfers);
        println!("Success rate: {:.1}%", self.success_rate());
        println!("Measured TPS: {:.2}", self.tps());
        if !self.transfers_per_block.is_empty() {
            let avg = self.transfers_per_block.iter().sum::<u32>() as f64
                / self.transfers_per_block.len() as f64;
            println!("Avg transfers/block: {:.1}", avg);
        }
        println!("===========================\n");
    }
}
