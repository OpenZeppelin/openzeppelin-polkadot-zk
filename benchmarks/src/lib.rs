//! Comprehensive TPS Benchmarks for Confidential Assets
//!
//! This crate provides SOTA benchmarking for confidential asset transfers:
//!
//! 1. **Native Verification Cost** - Raw ZK proof verification time
//! 2. **Block Filling Analysis** - How many txs fit in a 6s block (2s compute budget)
//! 3. **Incremental Cost Analysis** - Does cost change as block fills up?
//! 4. **TPS Projections** - Realistic throughput estimates
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all criterion benchmarks
//! cargo bench -p confidential-benchmarks
//!
//! # Generate TPS report
//! cargo run -p confidential-benchmarks --release
//! ```

pub mod verification;
pub mod block_sim;
pub mod tps;

use serde::{Deserialize, Serialize};

/// Block parameters for Polkadot parachains
pub mod block_params {
    /// Block time in milliseconds (6 seconds)
    pub const BLOCK_TIME_MS: u64 = 6000;

    /// Compute budget per block in milliseconds (2 seconds for normal extrinsics)
    /// Based on: WEIGHT_REF_TIME_PER_SECOND * 2 = 2 seconds total block weight
    /// But only 75% goes to normal dispatch (NORMAL_DISPATCH_RATIO)
    pub const COMPUTE_BUDGET_MS: u64 = 1500; // 2000 * 0.75

    /// Picoseconds per millisecond
    pub const PICOS_PER_MS: u64 = 1_000_000_000;

    /// Reference time unit (1 second in picoseconds)
    pub const WEIGHT_REF_TIME_PER_SECOND: u64 = 1_000_000_000_000;
}

/// Benchmark results for a single operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBenchmark {
    pub name: String,
    pub mean_ms: f64,
    pub std_dev_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub samples: usize,
}

/// Block filling analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFillingResult {
    pub operation: String,
    pub txs_per_block: u32,
    pub total_time_ms: f64,
    pub avg_time_per_tx_ms: f64,
    /// Does the Nth tx cost more than the 1st?
    pub incremental_costs: Vec<f64>,
    /// Percentage increase from first to last tx
    pub cost_increase_pct: f64,
}

/// Complete TPS report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpsReport {
    pub timestamp: String,
    pub hardware: HardwareInfo,
    pub verification_benchmarks: Vec<OperationBenchmark>,
    pub block_filling: Vec<BlockFillingResult>,
    pub tps_estimates: TpsEstimates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu: String,
    pub cores: usize,
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpsEstimates {
    /// Theoretical max based on verification time alone
    pub theoretical_max_tps: f64,
    /// Realistic TPS accounting for block filling overhead
    pub realistic_tps: f64,
    /// TPS for complete transfer (sender proof + receiver claim)
    pub complete_transfer_tps: f64,
    /// Comparison to ecosystem benchmarks
    pub ecosystem_comparison: EcosystemComparison,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemComparison {
    /// Polkadot relay chain TPS (standard transfers)
    pub polkadot_relay_tps: f64,
    /// Kusama stress test peak TPS
    pub kusama_peak_tps: f64,
    /// Solana confidential token TPS (if available)
    pub solana_confidential_tps: Option<f64>,
    /// Our confidential TPS as percentage of standard
    pub confidential_vs_standard_pct: f64,
}

impl TpsReport {
    pub fn print_summary(&self) {
        println!("\n========== TPS BENCHMARK REPORT ==========\n");
        println!("Hardware: {} ({} cores)", self.hardware.cpu, self.hardware.cores);
        println!("Generated: {}\n", self.timestamp);

        println!("--- Verification Benchmarks (Native) ---");
        for bench in &self.verification_benchmarks {
            println!(
                "  {}: {:.3}ms (±{:.3}ms)",
                bench.name, bench.mean_ms, bench.std_dev_ms
            );
        }

        println!("\n--- Block Filling Analysis ---");
        for bf in &self.block_filling {
            println!(
                "  {}: {} txs/block, {:.3}ms avg/tx, cost increase: {:.1}%",
                bf.operation, bf.txs_per_block, bf.avg_time_per_tx_ms, bf.cost_increase_pct
            );
        }

        println!("\n--- TPS Estimates ---");
        let tps = &self.tps_estimates;
        println!("  Theoretical Max (verify only): {:.0} TPS", tps.theoretical_max_tps);
        println!("  Realistic (with overhead):     {:.0} TPS", tps.realistic_tps);
        println!("  Complete Transfer (send+claim): {:.0} TPS", tps.complete_transfer_tps);

        println!("\n--- Ecosystem Comparison ---");
        let eco = &tps.ecosystem_comparison;
        println!("  Polkadot Standard TPS:    ~{:.0}", eco.polkadot_relay_tps);
        println!("  Kusama Peak (stress test): ~{:.0}", eco.kusama_peak_tps);
        println!(
            "  Confidential vs Standard:  {:.1}% (privacy has a cost)",
            eco.confidential_vs_standard_pct
        );

        println!("\n========== END REPORT ==========\n");
    }
}
