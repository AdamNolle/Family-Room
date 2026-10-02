#!/usr/bin/env bash
set -euo pipefail
TASK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$TASK_ROOT"
source "$TASK_ROOT/script/rust_env.sh"
rustup run 1.98.0 cargo build -p family-core --features bindings
mkdir -p Generated/Swift Generated/Kotlin
case "$(uname -s)" in
  Darwin) TASK_CORE_LIBRARY=target/debug/libfamily_core.dylib ;;
  Linux) TASK_CORE_LIBRARY=target/debug/libfamily_core.so ;;
  *) echo "Binding generation requires a supported macOS or Linux host." >&2; exit 1 ;;
esac
rustup run 1.98.0 cargo run -p family-core --features bindings --bin uniffi-bindgen -- generate --library "$TASK_CORE_LIBRARY" --language swift --out-dir Generated/Swift
rustup run 1.98.0 cargo run -p family-core --features bindings --bin uniffi-bindgen -- generate --library "$TASK_CORE_LIBRARY" --language kotlin --out-dir Generated/Kotlin
cp Generated/Swift/family_coreFFI.modulemap Generated/Swift/module.modulemap
