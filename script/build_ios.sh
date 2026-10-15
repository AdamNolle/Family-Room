#!/usr/bin/env bash
set -euo pipefail
TASK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$TASK_ROOT"
source "$TASK_ROOT/script/rust_env.sh"
./script/build_core.sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
IPHONEOS_DEPLOYMENT_TARGET=17.0 rustup run 1.98.0 cargo build -p family-core --release --target aarch64-apple-ios
IPHONEOS_DEPLOYMENT_TARGET=17.0 rustup run 1.98.0 cargo build -p family-core --release --target aarch64-apple-ios-sim
mkdir -p Generated/Apple
TASK_FRAMEWORK="$TASK_ROOT/Generated/Apple/FamilyCoreNative.xcframework"
if [[ -d "$TASK_FRAMEWORK" ]]; then rm -r "$TASK_FRAMEWORK"; fi
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libfamily_core.a -headers Generated/Swift \
  -library target/aarch64-apple-ios-sim/release/libfamily_core.a -headers Generated/Swift \
  -output "$TASK_FRAMEWORK"
xcodegen generate --spec platforms/apple/project.yml
xcodebuild -project platforms/apple/FamilyRoom.xcodeproj -scheme FamilyRoom \
  -destination 'generic/platform=iOS Simulator' -derivedDataPath /tmp/FamilyRoom-iOS-build \
  CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- ARCHS=arm64 ONLY_ACTIVE_ARCH=YES build
