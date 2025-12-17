use confidential_assets_primitives::ZkVerifier as _;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use zkhe_vectors::{
    ACCEPT_ENVELOPE, ASSET_ID_BYTES, RECEIVER_PK32, SENDER_PK32, TRANSFER_BUNDLE,
    TRANSFER_DELTA_COMM_32, TRANSFER_DELTA_CT_64, TRANSFER_FROM_OLD_COMM_32,
};
use zkhe_verifier::ZkheVerifier;

const IDENTITY_C32: [u8; 32] = [0u8; 32];

fn bench_transfer_verify(c: &mut Criterion) {
    let mut g = c.benchmark_group("verify_transfer_sent");
    g.throughput(Throughput::Elements(1));

    g.bench_function(BenchmarkId::from_parameter("transfer"), |b| {
        b.iter(|| {
            let (from_new, to_new) = ZkheVerifier::verify_transfer_sent(
                ASSET_ID_BYTES,
                &SENDER_PK32,
                &RECEIVER_PK32,
                &TRANSFER_FROM_OLD_COMM_32,
                &IDENTITY_C32,
                &TRANSFER_DELTA_CT_64,
                TRANSFER_BUNDLE,
            )
            .expect("transfer verify");
            black_box((from_new, to_new));
        });
    });

    g.finish();
}

fn bench_accept_verify(c: &mut Criterion) {
    let mut g = c.benchmark_group("verify_transfer_received");
    g.throughput(Throughput::Elements(1));

    g.bench_function(BenchmarkId::from_parameter("accept"), |b| {
        b.iter(|| {
            let (avail_new, pending_new) = ZkheVerifier::verify_transfer_received(
                ASSET_ID_BYTES,
                &RECEIVER_PK32,
                &IDENTITY_C32,
                &TRANSFER_DELTA_COMM_32,
                &[TRANSFER_DELTA_COMM_32],
                ACCEPT_ENVELOPE,
            )
            .expect("accept verify");
            black_box((avail_new, pending_new));
        });
    });

    g.finish();
}

criterion_group!(benches, bench_transfer_verify, bench_accept_verify);
criterion_main!(benches);
