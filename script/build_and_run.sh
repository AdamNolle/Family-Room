#!/usr/bin/env bash
set -euo pipefail
TASK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$TASK_ROOT"
MODE="${1:-run}"
TASK_BUILD_DIR="${FAMILY_ROOM_BUILD_DIR:-$HOME/Library/Caches/com.familyroom.build}"
TASK_BUNDLE="$TASK_BUILD_DIR/FamilyRoom.app"
family_room_mac_processes() {
  local TASK_CANDIDATE TASK_EXECUTABLE
  while IFS= read -r TASK_CANDIDATE; do
    [[ "$TASK_CANDIDATE" =~ ^[0-9]+$ ]] || continue
    TASK_EXECUTABLE="$(ps -p "$TASK_CANDIDATE" -o comm= 2>/dev/null || true)"
    if [[ "$TASK_EXECUTABLE" == "$TASK_BUNDLE/Contents/MacOS/FamilyRoom" ]]; then
      printf '%s\n' "$TASK_CANDIDATE"
    fi
  done < <(pgrep -x FamilyRoom || true)
}
if [[ "$MODE" != --build-only ]]; then
  while IFS= read -r TASK_CANDIDATE; do
    [[ -n "$TASK_CANDIDATE" ]] && kill "$TASK_CANDIDATE" 2>/dev/null || true
  done < <(family_room_mac_processes)
fi
./script/build_core.sh
swift build --build-system native --product FamilyRoom
TASK_BIN_DIR="$(swift build --build-system native --show-bin-path)"
mkdir -p "$TASK_BUNDLE/Contents/MacOS" "$TASK_BUNDLE/Contents/Frameworks"
ditto --norsrc --noextattr --noacl "$TASK_BIN_DIR/FamilyRoom" "$TASK_BUNDLE/Contents/MacOS/FamilyRoom"
ditto --norsrc --noextattr --noacl target/debug/libfamily_core.dylib "$TASK_BUNDLE/Contents/Frameworks/libfamily_core.dylib"
install_name_tool -id @rpath/libfamily_core.dylib "$TASK_BUNDLE/Contents/Frameworks/libfamily_core.dylib"
TASK_DYLIB_ID="$(otool -D target/debug/libfamily_core.dylib | tail -1)"
install_name_tool -change "$TASK_DYLIB_ID" @rpath/libfamily_core.dylib "$TASK_BUNDLE/Contents/MacOS/FamilyRoom"
install_name_tool -delete_rpath "$TASK_ROOT/target/debug" "$TASK_BUNDLE/Contents/MacOS/FamilyRoom" 2>/dev/null || true
install_name_tool -add_rpath @executable_path/../Frameworks "$TASK_BUNDLE/Contents/MacOS/FamilyRoom" 2>/dev/null || true
cat > "$TASK_BUNDLE/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>FamilyRoom</string>
<key>CFBundleIdentifier</key><string>com.familyroom.app</string>
<key>CFBundleName</key><string>Family Room</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
<key>NSPhotoLibraryUsageDescription</key><string>Import and automatically share your selected photos with your chosen Room.</string>
<key>NSMicrophoneUsageDescription</key><string>Record voice-over for your family films.</string>
</dict></plist>
PLIST
xattr -cr "$TASK_BUNDLE"
# Local Debug builds use an isolated development vault and ad hoc signing.
# Do not access a signing certificate's Keychain private key on every rebuild.
TASK_SIGN_IDENTITY="${FAMILY_ROOM_SIGN_IDENTITY:--}"
codesign --force --sign "$TASK_SIGN_IDENTITY" "$TASK_BUNDLE/Contents/Frameworks/libfamily_core.dylib"
codesign --force --sign "$TASK_SIGN_IDENTITY" "$TASK_BUNDLE"
codesign --verify --deep --strict "$TASK_BUNDLE"
# File Provider may immediately reapply FinderInfo inside a synced checkout.
# Keep the signed bundle outside it and expose a development link in dist.
mkdir -p "$TASK_ROOT/dist"
if [[ -L "$TASK_ROOT/dist/FamilyRoom.app" ]]; then
  rm "$TASK_ROOT/dist/FamilyRoom.app"
elif [[ -e "$TASK_ROOT/dist/FamilyRoom.app" ]]; then
  mv "$TASK_ROOT/dist/FamilyRoom.app" "$TASK_BUILD_DIR/Previous-FamilyRoom-$(date +%s).app"
fi
ln -s "$TASK_BUNDLE" "$TASK_ROOT/dist/FamilyRoom.app"
case "$MODE" in
  --build-only) ;;
  run) open -n "$TASK_BUNDLE" ;;
  --verify) open -n "$TASK_BUNDLE"; sleep 2; [[ -n "$(family_room_mac_processes)" ]] ;;
  --debug) lldb -- "$TASK_BUNDLE/Contents/MacOS/FamilyRoom" ;;
  --logs) open -n "$TASK_BUNDLE"; /usr/bin/log stream --info --style compact --predicate 'process == "FamilyRoom"' ;;
  --telemetry) open -n "$TASK_BUNDLE"; /usr/bin/log stream --info --style compact --predicate 'subsystem == "com.familyroom.app"' ;;
  *) echo "usage: $0 [run|--build-only|--verify|--debug|--logs|--telemetry]" >&2; exit 2 ;;
esac
