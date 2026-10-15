#!/usr/bin/env bash
# Unit tests via SwiftPM, then an iOS simulator build and a macOS build through Xcode.
set -euo pipefail
cd "$(dirname "$0")/.."
swift test --quiet
./script/build_ios.sh >/dev/null
./script/build_and_run.sh --build-only >/dev/null
echo "verify: ok"
