# Benchmarking Results

Apple M4 Pro:
```
Benchmark                                 mean_ms    proofs/6s
verify_transfer                           1.373         4370
verify_claim                              2.360         2542
```
On production hardware these results conservatively translate to ~1k confidential transfers per second. On-chain verification is the main bottleneck for ZK El Gamal.

## Steps To Reproduce

1. Run `cargo bench --verify` for zkhe-verifier.
2. Run `cargo run` for this crate to reproduce results locally.
