use std::{env, fs, path::PathBuf};
use zkhe_prover::bench_vectors::some_valid_proofs; // move your write_proofs_to_file() into prover::bench_vectors

fn main() {
    // Write into zkhe-vectors/src/generated.rs
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let dst = workspace.join("zkhe-vectors/src/generated.rs");
    let code = some_valid_proofs();
    fs::write(&dst, code).expect("write vectors");
    eprintln!("Wrote {}", dst.display());
}
