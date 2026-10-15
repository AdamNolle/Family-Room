#!/usr/bin/env bash
set -euo pipefail
TASK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$TASK_ROOT"
source "$TASK_ROOT/script/rust_env.sh"
if [[ -z "${ANDROID_HOME:-}" && -n "${ANDROID_SDK_ROOT:-}" ]]; then
  ANDROID_HOME="$ANDROID_SDK_ROOT"
fi
if [[ -z "${ANDROID_HOME:-}" && -d "${HOME:-}/Library/Android/sdk" ]]; then
  ANDROID_HOME="${HOME}/Library/Android/sdk"
fi
if [[ -n "${ANDROID_HOME:-}" ]]; then
  export ANDROID_HOME
  export ANDROID_SDK_ROOT="$ANDROID_HOME"
fi
if [[ -z "${ANDROID_NDK_HOME:-}" && -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME/ndk" ]]; then
  for candidate in "$ANDROID_HOME"/ndk/*; do
    if [[ -x "$candidate/ndk-build" ]]; then
      ANDROID_NDK_HOME="$candidate"
    fi
  done
fi
: "${ANDROID_NDK_HOME:?Install an Android NDK or set ANDROID_NDK_HOME}"
export ANDROID_NDK_HOME
if [[ -z "${JAVA_HOME:-}" ]]; then
  if [[ -x /opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home/bin/java ]]; then
    JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
  elif [[ -x /usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home/bin/java ]]; then
    JAVA_HOME=/usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
  elif [[ -x /usr/libexec/java_home ]]; then
    JAVA_HOME="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"
  fi
fi
if [[ -n "${JAVA_HOME:-}" && -x "$JAVA_HOME/bin/java" ]]; then
  export JAVA_HOME
  export PATH="$JAVA_HOME/bin:$PATH"
fi
rustup target add aarch64-linux-android x86_64-linux-android
TASK_HOST="$(uname -s | tr '[:upper:]' '[:lower:]')-x86_64"
TASK_TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$TASK_HOST/bin"
for TASK_TARGET in aarch64-linux-android x86_64-linux-android; do
  if [[ "$TASK_TARGET" == aarch64-linux-android ]]; then TASK_ABI=arm64-v8a; else TASK_ABI=x86_64; fi
  TASK_LINKER="$TASK_TOOLCHAIN/${TASK_TARGET}26-clang"
  TASK_LINKER_VAR="CARGO_TARGET_$(echo "$TASK_TARGET" | tr '[:lower:]-' '[:upper:]_')_LINKER"
  env "$TASK_LINKER_VAR=$TASK_LINKER" CC="$TASK_LINKER" AR="$TASK_TOOLCHAIN/llvm-ar" \
    rustup run 1.98.0 cargo build -p family-core --target "$TASK_TARGET" --release
  mkdir -p "platforms/android/app/src/main/jniLibs/$TASK_ABI"
  cp "target/$TASK_TARGET/release/libfamily_core.so" "platforms/android/app/src/main/jniLibs/$TASK_ABI/"
done
./script/build_core.sh
cd platforms/android
./gradlew testDebugUnitTest assembleDebug assembleQaAndroidTest
TASK_APP_BUILD_DIR="$(./gradlew --quiet printAppBuildDirectory)"
mkdir -p "$TASK_ROOT/dist/android"
cp "$TASK_APP_BUILD_DIR/outputs/apk/debug/app-debug.apk" "$TASK_ROOT/dist/android/FamilyRoom-debug.apk"
