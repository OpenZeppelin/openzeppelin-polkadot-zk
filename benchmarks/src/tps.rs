//! TPS calculation and ecosystem comparison
//!
//! Calculates realistic TPS estimates and compares to ecosystem benchmarks.

use crate::block_params::*;
use crate::{BlockFillingResult, EcosystemComparison, OperationBenchmark, TpsEstimates};

/// Ecosystem benchmark data (from public sources)
pub mod ecosystem_data {
    // Source: https://github.com/paritytech/polkadot-stps
    // Source: https://chainspect.app/chain/polkadot-ecosystem

    /// Polkadot relay chain measured TPS (standard balance transfers)
    pub const POLKADOT_MEASURED_TPS: f64 = 1000.0;

    /// Kusama stress test peak TPS (2024 "Spammening")
    /// Source: Parity/Amforc joint test - 82,171 TPS with batching, 10,547 non-batch
    pub const KUSAMA_PEAK_TPS: f64 = 10_547.0;

    /// Kusama theoretical max TPS (all cores utilized)
    pub const KUSAMA_THEORETICAL_MAX_TPS: f64 = 623_230.0;

    /// Solana confidential token - estimated based on their SDK
    /// Note: Solana's confidential tokens use similar ZK-ElGamal scheme
    pub const SOLANA_CONFIDENTIAL_TPS: Option<f64> = None; // No public benchmark data yet
}

/// Calculate TPS estimates from benchmark results
pub fn calculate_tps_estimates(
    verification_benchmarks: &[OperationBenchmark],
    block_filling: &[BlockFillingResult],
) -> TpsEstimates {
    // Find transfer and accept benchmarks
    let transfer_bench = verification_benchmarks
        .iter()
        .find(|b| b.name.contains("sent") || b.name.contains("transfer"))
        .expect("transfer benchmark required");

    let accept_bench = verification_benchmarks
        .iter()
        .find(|b| b.name.contains("received") || b.name.contains("accept") || b.name.contains("claim"))
        .expect("accept benchmark required");

    // Theoretical max: just verification time, no overhead
    let theoretical_transfer_tps = (BLOCK_TIME_MS as f64) / transfer_bench.mean_ms;
    let _theoretical_accept_tps = (BLOCK_TIME_MS as f64) / accept_bench.mean_ms;

    // Use block filling results for realistic estimates
    let transfer_filling = block_filling
        .iter()
        .find(|b| b.operation.contains("transfer_sent") || b.operation == "verify_transfer_sent")
        .map(|b| b.txs_per_block as f64 / (BLOCK_TIME_MS as f64 / 1000.0))
        .unwrap_or(theoretical_transfer_tps);

    let complete_filling = block_filling
        .iter()
        .find(|b| b.operation.contains("complete"))
        .map(|b| b.txs_per_block as f64 / (BLOCK_TIME_MS as f64 / 1000.0))
        .unwrap_or(transfer_filling / 2.0);

    // Calculate ecosystem comparison
    let confidential_vs_standard = (transfer_filling / ecosystem_data::POLKADOT_MEASURED_TPS) * 100.0;

    TpsEstimates {
        theoretical_max_tps: theoretical_transfer_tps,
        realistic_tps: transfer_filling,
        complete_transfer_tps: complete_filling,
        ecosystem_comparison: EcosystemComparison {
            polkadot_relay_tps: ecosystem_data::POLKADOT_MEASURED_TPS,
            kusama_peak_tps: ecosystem_data::KUSAMA_PEAK_TPS,
            solana_confidential_tps: ecosystem_data::SOLANA_CONFIDENTIAL_TPS,
            confidential_vs_standard_pct: confidential_vs_standard,
        },
    }
}

/// Generate markdown table comparing TPS across operations
pub fn generate_tps_comparison_table(estimates: &TpsEstimates) -> String {
    let mut table = String::new();

    table.push_str("| Metric | Value | Notes |\n");
    table.push_str("|--------|-------|-------|\n");
    table.push_str(&format!(
        "| Theoretical Max TPS | {:.0} | Verification time only |\n",
        estimates.theoretical_max_tps
    ));
    table.push_str(&format!(
        "| Realistic TPS (sender phase) | {:.0} | With block filling overhead |\n",
        estimates.realistic_tps
    ));
    table.push_str(&format!(
        "| Complete Transfer TPS | {:.0} | Send + Claim (two proofs) |\n",
        estimates.complete_transfer_tps
    ));
    table.push_str(&format!(
        "| Polkadot Standard TPS | {:.0} | Balance transfers (reference) |\n",
        estimates.ecosystem_comparison.polkadot_relay_tps
    ));
    table.push_str(&format!(
        "| Kusama Peak TPS | {:.0} | Stress test (non-batch) |\n",
        estimates.ecosystem_comparison.kusama_peak_tps
    ));
    table.push_str(&format!(
        "| Confidential vs Standard | {:.1}% | Privacy overhead |\n",
        estimates.ecosystem_comparison.confidential_vs_standard_pct
    ));

    table
}
