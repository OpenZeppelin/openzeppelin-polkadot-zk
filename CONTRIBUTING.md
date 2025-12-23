# Contributing to Polkadot Confidential Assets

We really appreciate and value contributions. Make sure to read these guidelines before starting to contribute.

## Getting Started

Before developing, please create an [issue](https://github.com/OpenZeppelin/openzeppelin-polkadot-zk/issues/new) to discuss what you'd like to work on. This helps ensure your contribution will be accepted.

Look for issues labeled "good first issue" if you're new to the project.

## Development Setup

### Prerequisites

```bash
# Rust toolchain
rustup default stable
rustup target add wasm32-unknown-unknown

# For WASM builds (macOS)
brew install llvm
export CC_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/clang"
export AR_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/llvm-ar"
```

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test --workspace
```

## Pull Request Process

1. **Sync your fork** with the upstream `main` branch
2. **Create a feature branch** from `main` (e.g., `fix/description-#issue-number`)
3. **Make your changes** following the code style guidelines below
4. **Run tests** and ensure they pass
5. **Submit a PR** linking to the relevant issue with "Fixes #123" or "Resolves #123"

## Code Style

- Run `cargo fmt --all` before committing
- Ensure `cargo clippy` passes without warnings
- Follow Rust conventions and idioms
- Include documentation for public functions and modules

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
feat: add new transfer verification
fix: correct balance calculation
docs: update API documentation
test: add integration tests for bridge
```

## Questions?

Feel free to open an issue if you have questions or need guidance.
