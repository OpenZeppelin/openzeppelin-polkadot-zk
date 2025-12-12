//! Criterion benchmarks for ZK verification
//!
//! Run with: cargo bench -p confidential-benchmarks

use confidential_assets_primitives::ZkVerifier;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use zkhe_vectors::*;
use zkhe_verifier::ZkheVerifier;

const IDENTITY_C32: [u8; 32] = [0u8; 32];

fn bench_verify_transfer_sent(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_transfer_sent");
    group.throughput(Throughput::Elements(1));
    group.sample_size(100);

    group.bench_function(BenchmarkId::from_parameter("single"), |b| {
        b.iter(|| {
            let (from_new, to_new) = ZkheVerifier::verify_transfer_sent(
                black_box(ASSET_ID_BYTES),
                black_box(&SENDER_PK32),
                black_box(&RECEIVER_PK32),
                black_box(&TRANSFER_FROM_OLD_COMM_32),
                black_box(&IDENTITY_C32),
                black_box(&TRANSFER_DELTA_CT_64),
                black_box(TRANSFER_BUNDLE),
            )
            .expect("verify");
            black_box((from_new, to_new))
        });
    });

    group.finish();
}

fn bench_verify_transfer_received(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_transfer_received");
    group.throughput(Throughput::Elements(1));
    group.sample_size(100);

    group.bench_function(BenchmarkId::from_parameter("single"), |b| {
        b.iter(|| {
            let (avail_new, pending_new) = ZkheVerifier::verify_transfer_received(
                black_box(ASSET_ID_BYTES),
                black_box(&RECEIVER_PK32),
                black_box(&IDENTITY_C32),
                black_box(&TRANSFER_DELTA_COMM_32),
                black_box(&[TRANSFER_DELTA_COMM_32]),
                black_box(ACCEPT_ENVELOPE),
            )
            .expect("verify");
            black_box((avail_new, pending_new))
        });
    });

    group.finish();
}

fn bench_complete_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_transfer");
    group.throughput(Throughput::Elements(1));
    group.sample_size(50); // Fewer samples since this is 2 proofs

    group.bench_function(BenchmarkId::from_parameter("send_plus_claim"), |b| {
        b.iter(|| {
            // Sender phase
            let (from_new, to_new) = ZkheVerifier::verify_transfer_sent(
                black_box(ASSET_ID_BYTES),
                black_box(&SENDER_PK32),
                black_box(&RECEIVER_PK32),
                black_box(&TRANSFER_FROM_OLD_COMM_32),
                black_box(&IDENTITY_C32),
                black_box(&TRANSFER_DELTA_CT_64),
                black_box(TRANSFER_BUNDLE),
            )
            .expect("verify");

            // Receiver phase
            let (avail_new, pending_new) = ZkheVerifier::verify_transfer_received(
                black_box(ASSET_ID_BYTES),
                black_box(&RECEIVER_PK32),
                black_box(&IDENTITY_C32),
                black_box(&TRANSFER_DELTA_COMM_32),
                black_box(&[TRANSFER_DELTA_COMM_32]),
                black_box(ACCEPT_ENVELOPE),
            )
            .expect("verify");

            black_box((from_new, to_new, avail_new, pending_new))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_verify_transfer_sent,
    bench_verify_transfer_received,
    bench_complete_transfer,
);
criterion_main!(benches);
