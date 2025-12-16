//! TPS Report Generator
//!
//! Runs all benchmarks and generates a comprehensive TPS report.
//!
//! ## Usage
//!
//! ```bash
//! # Run with release optimizations for accurate numbers
//! cargo run -p confidential-benchmarks --release
//!
//! # Save report to file
//! cargo run -p confidential-benchmarks --release > tps_report.json
//! ```

use confidential_benchmarks::{
    HardwareInfo, OperationBenchmark, TpsReport, block_sim, tps, verification,
};

fn main() {
    println!("Confidential Assets TPS Benchmark Suite\n");
    println!("========================================\n");

    // Detect hardware
    let hardware = detect_hardware();
    println!("Hardware: {} ({} cores)\n", hardware.cpu, hardware.cores);

    // Run verification benchmarks
    println!("Phase 1: Native Verification Benchmarks");
    println!("----------------------------------------");
    let iterations = 100;
    let stats = verification::benchmark_verification(iterations);

    println!(
        "  verify_transfer_sent:     {:.3}ms ± {:.3}ms (p99: {:.3}ms)",
        stats.transfer.mean_ms, stats.transfer.std_dev_ms, stats.transfer.p99_ms
    );
    println!(
        "  verify_transfer_received: {:.3}ms ± {:.3}ms (p99: {:.3}ms)",
        stats.accept.mean_ms, stats.accept.std_dev_ms, stats.accept.p99_ms
    );

    let verification_benchmarks = vec![
        OperationBenchmark {
            name: "verify_transfer_sent".to_string(),
            mean_ms: stats.transfer.mean_ms,
            std_dev_ms: stats.transfer.std_dev_ms,
            min_ms: stats.transfer.min_ms,
            max_ms: stats.transfer.max_ms,
            samples: stats.transfer.samples,
        },
        OperationBenchmark {
            name: "verify_transfer_received".to_string(),
            mean_ms: stats.accept.mean_ms,
            std_dev_ms: stats.accept.std_dev_ms,
            min_ms: stats.accept.min_ms,
            max_ms: stats.accept.max_ms,
            samples: stats.accept.samples,
        },
    ];

    // Run block filling simulations
    println!("\nPhase 2: Block Filling Analysis");
    println!("-------------------------------");
    let block_filling = block_sim::run_all_block_simulations();

    for bf in &block_filling {
        println!(
            "  {}: {} txs/block, avg {:.3}ms/tx, cost drift: {:.1}%",
            bf.operation, bf.txs_per_block, bf.avg_time_per_tx_ms, bf.cost_increase_pct
        );
    }

    // Calculate TPS estimates
    println!("\nPhase 3: TPS Calculations");
    println!("-------------------------");
    let tps_estimates = tps::calculate_tps_estimates(&verification_benchmarks, &block_filling);

    println!(
        "  Theoretical Max TPS:       {:.0}",
        tps_estimates.theoretical_max_tps
    );
    println!(
        "  Realistic TPS:             {:.0}",
        tps_estimates.realistic_tps
    );
    println!(
        "  Complete Transfer TPS:     {:.0}",
        tps_estimates.complete_transfer_tps
    );
    println!(
        "  vs Polkadot Standard:      {:.1}%",
        tps_estimates
            .ecosystem_comparison
            .confidential_vs_standard_pct
    );

    // Generate report
    let report = TpsReport {
        timestamp: chrono_lite_timestamp(),
        hardware,
        verification_benchmarks,
        block_filling: block_filling
            .into_iter()
            .map(|bf| confidential_benchmarks::BlockFillingResult {
                operation: bf.operation,
                txs_per_block: bf.txs_per_block,
                total_time_ms: bf.total_time_ms,
                avg_time_per_tx_ms: bf.avg_time_per_tx_ms,
                incremental_costs: bf.incremental_costs,
                cost_increase_pct: bf.cost_increase_pct,
            })
            .collect(),
        tps_estimates,
    };

    // Print full report
    report.print_summary();

    // Output JSON for CI/automation
    println!("\n--- JSON Report ---");
    // Strip incremental_costs for cleaner output (can be very large)
    let mut clean_report = report.clone();
    for bf in &mut clean_report.block_filling {
        bf.incremental_costs = vec![]; // Clear for JSON output
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&clean_report).unwrap_or_else(|_| "JSON error".to_string())
    );
}

fn detect_hardware() -> HardwareInfo {
    let cpu = std::env::var("CPU_MODEL")
        .or_else(|_| {
            // Try to detect CPU on macOS/Linux
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("sysctl")
                    .args(["-n", "machdep.cpu.brand_string"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .ok_or(std::env::VarError::NotPresent)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Err(std::env::VarError::NotPresent)
            }
        })
        .unwrap_or_else(|_| "Unknown CPU".to_string());

    let cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let os = std::env::consts::OS.to_string();

    HardwareInfo { cpu, cores, os }
}

fn chrono_lite_timestamp() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("unix:{}", duration.as_secs())
}
