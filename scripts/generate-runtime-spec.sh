#!/bin/bash

# Mac mappings
export CC_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/clang"
export AR_wasm32_unknown_unknown="/opt/homebrew/opt/llvm/bin/llvm-ar"
export CC_wasm32v1_none="/opt/homebrew/opt/llvm/bin/clang"
export AR_wasm32v1_none="/opt/homebrew/opt/llvm/bin/llvm-ar"
# Build in release
cargo build --release

chain-spec-builder create -t development \
--relay-chain paseo \
--para-id 1001 \
--runtime ../../target/release/wbuild/asset-hub-runtime/asset_hub_runtime.compact.compressed.wasm \
named-preset development
