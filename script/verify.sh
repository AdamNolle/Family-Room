#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source script/rust_env.sh
rustup run 1.98.0 cargo fmt --all -- --check
rustup run 1.98.0 cargo clippy --workspace --all-targets --all-features -- -D warnings
rustup run 1.98.0 cargo test --workspace --all-features
./script/build_core.sh
# Native SwiftPM avoids Finder metadata being carried into an Xcode test bundle.
swift test --build-system native --scratch-path "${TMPDIR:-/tmp}/family-room-swift-tests"
