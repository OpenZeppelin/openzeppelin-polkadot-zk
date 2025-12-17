//! Native ZK verification benchmarks
//!
//! Measures raw proof verification time without any Substrate/WASM overhead.
//! This represents the theoretical minimum time for each operation.

use confidential_assets_primitives::{ZeroNetworkId, ZkVerifier};
use zkhe_vectors::*;
use zkhe_verifier::ZkheVerifier;

type Verifier = ZkheVerifier<ZeroNetworkId>;

const IDENTITY_C32: [u8; 32] = [0u8; 32];

/// Verify a sender transfer proof (Phase 1)
/// Returns (from_new_commitment, to_new_commitment)
pub fn verify_transfer_sent() -> (Vec<u8>, Vec<u8>) {
    Verifier::verify_transfer_sent(
        &ASSET_ID_BYTES,
        &SENDER_PK32,
        &RECEIVER_PK32,
        &TRANSFER_FROM_OLD_COMM_32,
        &IDENTITY_C32,
        &TRANSFER_DELTA_CT_64,
        TRANSFER_BUNDLE,
    )
    .expect("transfer verify should succeed")
}

/// Verify a receiver acceptance proof (Phase 2)
/// Returns (avail_new_commitment, pending_new_commitment)
pub fn verify_transfer_received() -> (Vec<u8>, Vec<u8>) {
    Verifier::verify_transfer_received(
        &ASSET_ID_BYTES,
        &RECEIVER_PK32,
        &IDENTITY_C32,
        &TRANSFER_DELTA_COMM_32,
        &[TRANSFER_DELTA_COMM_32],
        ACCEPT_ENVELOPE,
    )
    .expect("accept verify should succeed")
}

/// Benchmark verification N times and return timing stats
pub fn benchmark_verification(iterations: usize) -> VerificationStats {
    use std::time::Instant;

    let mut transfer_times = Vec::with_capacity(iterations);
    let mut accept_times = Vec::with_capacity(iterations);

    // Warmup
    for _ in 0..10 {
        let _ = verify_transfer_sent();
        let _ = verify_transfer_received();
    }

    // Actual measurements
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = verify_transfer_sent();
        transfer_times.push(start.elapsed().as_secs_f64() * 1000.0);

        let start = Instant::now();
        let _ = verify_transfer_received();
        accept_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    VerificationStats {
        transfer: compute_stats(&transfer_times),
        accept: compute_stats(&accept_times),
    }
}

#[derive(Debug, Clone)]
pub struct VerificationStats {
    pub transfer: TimingStats,
    pub accept: TimingStats,
}

#[derive(Debug, Clone)]
pub struct TimingStats {
    pub mean_ms: f64,
    pub std_dev_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub samples: usize,
}

fn compute_stats(times: &[f64]) -> TimingStats {
    let n = times.len();
    if n == 0 {
        return TimingStats {
            mean_ms: 0.0,
            std_dev_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            samples: 0,
        };
    }

    let mean = times.iter().sum::<f64>() / n as f64;
    let variance = times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    let mut sorted = times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    TimingStats {
        mean_ms: mean,
        std_dev_ms: std_dev,
        min_ms: sorted[0],
        max_ms: sorted[n - 1],
        p50_ms: sorted[n / 2],
        p95_ms: sorted[((n as f64 * 0.95) as usize).min(n.saturating_sub(1))],
        p99_ms: sorted[((n as f64 * 0.99) as usize).min(n.saturating_sub(1))],
        samples: n,
    }
}
