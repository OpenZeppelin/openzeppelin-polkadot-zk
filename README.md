# Polkadot Confidential Assets Framework

**WARNING: This is beta software that has NOT been audited and is NOT ready for production. Use at your own risk!**

Rust libraries for verifying computation executed on encrypted data directly on-chain, leveraging Polkadot's interoperability and Zero-Knowledge Proofs for privacy and verifiability.

**Pallets**
1. `pallet-confidential-assets`: confidential transfers interface following IERC7984 by OpenZeppelin, generic over cryptographic backend (ZK, FHE, TEE)
2. `pallet-confidential-bridge`: confidential cross-chain transfers
3. `pallet-confidential-escrow`: confidential asset escrow management
4. `pallet-zkhe`: encrypted balances stored on-chain post verification by generic ZK backend
5. `pallet-operators`: operator permissions registry for IERC 7984
6. `pallet-acl`: ACL permissions storage for IERC 7984

**Prover+Verifier**
1. `zkhe-prover`: client library to run off-chain for generating valid Zero Knowledge Proofs expected by on-chain verifier
2. `zkhe-verifier`: no_std verifier library to run on-chain for verifying valid Zero Knowledge Proofs expected by off-chain prover

## Examples

Read [the extension pallets](./book/examples/) for examples leveraging and extending the confidential assets framework.

## References

* [OpenZeppelin Confidential Contracts Standard](https://github.com/OpenZeppelin/openzeppelin-confidential-contracts/blob/master/contracts/interfaces/IERC7984.sol)
* [Solana Confidential Balances Overview](https://www.solana-program.com/docs/confidential-balances/overview)

## Development

### Pre-commit Hooks

This project uses [pre-commit](https://pre-commit.com/) to run checks before each commit. The hooks align with the CI workflow.

**Setup:**

```bash
# Install pre-commit (if not already installed)
pip install pre-commit

# Install toml-sort for Cargo.toml formatting
cargo install --git https://github.com/4meta5/toml_sort

# Install the git hooks
pre-commit install

# (Optional) Run hooks on all files
pre-commit run --all-files
```

**Hooks include:**
- `cargo fmt` - Rust code formatting
- `toml-sort` - Cargo.toml file sorting
- `cargo check` - Compilation error checking
- Standard file checks (trailing whitespace, YAML/TOML validation, etc.)

### Manual Checks

```bash
# Format Rust code
cargo fmt --all

# Sort Cargo.toml files
./scripts/toml-sort.sh

# Run tests
cargo nextest run --release

# Check documentation
cargo doc --release --locked --all --no-deps
```

## Security Policy

Please report any security issues you find to <security@openzeppelin.com>.
