use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Estimate {
    mean: Mean,
}
#[derive(Deserialize)]
struct Mean {
    point_estimate: f64, // nanoseconds
}

// Must run benchmarks before running this estimator by running cargo bench --verify for zkhe-verifier crate
fn main() {
    let block_ms = 6000.0; // 6-second block
    let paths = [
        "../../target/criterion/verify_transfer_sent/transfer/new/estimates.json",
        "../../target/criterion/verify_transfer_received/accept/new/estimates.json",
    ];

    println!("{:<36} {:>12} {:>12}", "Benchmark", "mean_ms", "proofs/6s");

    for path in paths {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(e) = serde_json::from_str::<Estimate>(&data) {
                let mean_ns = e.mean.point_estimate;
                let mean_ms = mean_ns / 1e6;
                let proofs_per_block = (block_ms / mean_ms) as u64;
                println!("{:<36} {:>12.3} {:>12}", path, mean_ms, proofs_per_block);
            } else {
                println!("{path}: parse error");
            }
        } else {
            println!("{path}: Benchmark results NOT found. Run benchmarks `cargo bench --verify` for zkhe-verifier");
        }
    }
}
