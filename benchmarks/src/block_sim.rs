//! Block filling simulation
//!
//! Simulates filling a block with confidential transactions to measure:
//! 1. How many txs fit in the compute budget
//! 2. Whether verification cost increases as block fills (cache effects, etc.)
//! 3. Realistic TPS accounting for all overhead

use crate::block_params::*;
use crate::verification::{verify_transfer_received, verify_transfer_sent};
use crate::BlockFillingResult;
use std::time::Instant;

/// Simulate filling a block with transfer verifications
/// Returns detailed timing for each transaction
pub fn simulate_block_filling_transfer(max_txs: usize) -> BlockFillingResult {
    let mut times = Vec::with_capacity(max_txs);
    let mut total_ms = 0.0;

    // Fill block until we hit compute budget
    for _ in 0..max_txs {
        let start = Instant::now();
        let _ = verify_transfer_sent();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        times.push(elapsed_ms);
        total_ms += elapsed_ms;

        // Stop if we've exceeded the compute budget
        if total_ms > COMPUTE_BUDGET_MS as f64 {
            break;
        }
    }

    let txs_per_block = times.len() as u32;
    let avg_time = if times.is_empty() {
        0.0
    } else {
        total_ms / times.len() as f64
    };

    // Calculate cost increase from first to last tx
    let cost_increase_pct = if times.len() > 1 {
        let first_avg = times[..5.min(times.len())].iter().sum::<f64>() / 5.0f64.min(times.len() as f64);
        let last_avg = times[times.len().saturating_sub(5)..]
            .iter()
            .sum::<f64>()
            / 5.0f64.min(times.len() as f64);
        ((last_avg - first_avg) / first_avg) * 100.0
    } else {
        0.0
    };

    BlockFillingResult {
        operation: "verify_transfer_sent".to_string(),
        txs_per_block,
        total_time_ms: total_ms,
        avg_time_per_tx_ms: avg_time,
        incremental_costs: times,
        cost_increase_pct,
    }
}

/// Simulate filling a block with accept verifications
pub fn simulate_block_filling_accept(max_txs: usize) -> BlockFillingResult {
    let mut times = Vec::with_capacity(max_txs);
    let mut total_ms = 0.0;

    for _ in 0..max_txs {
        let start = Instant::now();
        let _ = verify_transfer_received();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        times.push(elapsed_ms);
        total_ms += elapsed_ms;

        if total_ms > COMPUTE_BUDGET_MS as f64 {
            break;
        }
    }

    let txs_per_block = times.len() as u32;
    let avg_time = if times.is_empty() {
        0.0
    } else {
        total_ms / times.len() as f64
    };

    let cost_increase_pct = if times.len() > 1 {
        let first_avg = times[..5.min(times.len())].iter().sum::<f64>() / 5.0f64.min(times.len() as f64);
        let last_avg = times[times.len().saturating_sub(5)..]
            .iter()
            .sum::<f64>()
            / 5.0f64.min(times.len() as f64);
        ((last_avg - first_avg) / first_avg) * 100.0
    } else {
        0.0
    };

    BlockFillingResult {
        operation: "verify_transfer_received".to_string(),
        txs_per_block,
        total_time_ms: total_ms,
        avg_time_per_tx_ms: avg_time,
        incremental_costs: times,
        cost_increase_pct,
    }
}

/// Simulate complete transfers (send + receive as atomic pair)
pub fn simulate_block_filling_complete_transfer(max_txs: usize) -> BlockFillingResult {
    let mut times = Vec::with_capacity(max_txs);
    let mut total_ms = 0.0;

    for _ in 0..max_txs {
        let start = Instant::now();
        // A complete transfer requires both sender and receiver proofs
        let _ = verify_transfer_sent();
        let _ = verify_transfer_received();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        times.push(elapsed_ms);
        total_ms += elapsed_ms;

        if total_ms > COMPUTE_BUDGET_MS as f64 {
            break;
        }
    }

    let txs_per_block = times.len() as u32;
    let avg_time = if times.is_empty() {
        0.0
    } else {
        total_ms / times.len() as f64
    };

    let cost_increase_pct = if times.len() > 1 {
        let first_avg = times[..5.min(times.len())].iter().sum::<f64>() / 5.0f64.min(times.len() as f64);
        let last_avg = times[times.len().saturating_sub(5)..]
            .iter()
            .sum::<f64>()
            / 5.0f64.min(times.len() as f64);
        ((last_avg - first_avg) / first_avg) * 100.0
    } else {
        0.0
    };

    BlockFillingResult {
        operation: "complete_transfer (send+claim)".to_string(),
        txs_per_block,
        total_time_ms: total_ms,
        avg_time_per_tx_ms: avg_time,
        incremental_costs: times,
        cost_increase_pct,
    }
}

/// Run all block filling simulations
pub fn run_all_block_simulations() -> Vec<BlockFillingResult> {
    const MAX_TXS: usize = 5000; // Upper bound for simulation

    println!("Running block filling simulations...");

    let transfer = simulate_block_filling_transfer(MAX_TXS);
    println!(
        "  Transfer: {} txs fit in block ({:.1}ms budget)",
        transfer.txs_per_block, COMPUTE_BUDGET_MS
    );

    let accept = simulate_block_filling_accept(MAX_TXS);
    println!("  Accept: {} txs fit in block", accept.txs_per_block);

    let complete = simulate_block_filling_complete_transfer(MAX_TXS);
    println!(
        "  Complete transfer: {} txs fit in block",
        complete.txs_per_block
    );

    vec![transfer, accept, complete]
}
