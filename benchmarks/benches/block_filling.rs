//! Block filling benchmarks
//!
//! Measures how verification cost changes as more txs are added to a block.
//! This tests for cache effects, memory pressure, and other runtime behaviors.

use confidential_assets_primitives::ZkVerifier;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use zkhe_vectors::*;
use zkhe_verifier::ZkheVerifier;

const IDENTITY_C32: [u8; 32] = [0u8; 32];

/// Benchmark N sequential verifications to measure cost drift
fn bench_sequential_verifications(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_verifications");

    // Test different batch sizes to see if cost per tx changes
    for batch_size in [1, 10, 50, 100, 200] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_function(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            |b| {
                b.iter(|| {
                    for _ in 0..batch_size {
                        let (from_new, to_new) = ZkheVerifier::verify_transfer_sent(
                            black_box(&ASSET_ID_BYTES),
                            black_box(&SENDER_PK32),
                            black_box(&RECEIVER_PK32),
                            black_box(&TRANSFER_FROM_OLD_COMM_32),
                            black_box(&IDENTITY_C32),
                            black_box(&TRANSFER_DELTA_CT_64),
                            black_box(TRANSFER_BUNDLE),
                        )
                        .expect("verify");
                        black_box((from_new, to_new));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark interleaved send/receive to simulate realistic block pattern
fn bench_interleaved_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("interleaved_pattern");

    // Simulate blocks with mixed tx types
    for pairs in [5, 10, 25, 50] {
        group.throughput(Throughput::Elements((pairs * 2) as u64));

        group.bench_function(
            BenchmarkId::from_parameter(format!("{}_pairs", pairs)),
            |b| {
                b.iter(|| {
                    for _ in 0..pairs {
                        // Sender proof
                        let _ = ZkheVerifier::verify_transfer_sent(
                            black_box(&ASSET_ID_BYTES),
                            black_box(&SENDER_PK32),
                            black_box(&RECEIVER_PK32),
                            black_box(&TRANSFER_FROM_OLD_COMM_32),
                            black_box(&IDENTITY_C32),
                            black_box(&TRANSFER_DELTA_CT_64),
                            black_box(TRANSFER_BUNDLE),
                        )
                        .expect("verify");

                        // Receiver proof (could be for a different transfer)
                        let _ = ZkheVerifier::verify_transfer_received(
                            black_box(&ASSET_ID_BYTES),
                            black_box(&RECEIVER_PK32),
                            black_box(&IDENTITY_C32),
                            black_box(&TRANSFER_DELTA_COMM_32),
                            black_box(&[TRANSFER_DELTA_COMM_32]),
                            black_box(ACCEPT_ENVELOPE),
                        )
                        .expect("verify");
                    }
                });
            },
        );
    }

    group.finish();
}

/// Measure memory allocation patterns during batch verification
fn bench_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure");
    group.sample_size(20); // Fewer samples for large batches

    // Large batch to stress memory
    for batch_size in [500, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_function(
            BenchmarkId::from_parameter(format!("transfers_{}", batch_size)),
            |b| {
                b.iter(|| {
                    for _ in 0..batch_size {
                        let _ = ZkheVerifier::verify_transfer_sent(
                            black_box(&ASSET_ID_BYTES),
                            black_box(&SENDER_PK32),
                            black_box(&RECEIVER_PK32),
                            black_box(&TRANSFER_FROM_OLD_COMM_32),
                            black_box(&IDENTITY_C32),
                            black_box(&TRANSFER_DELTA_CT_64),
                            black_box(TRANSFER_BUNDLE),
                        )
                        .expect("verify");
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_verifications,
    bench_interleaved_pattern,
    bench_memory_pressure,
);
criterion_main!(benches);
