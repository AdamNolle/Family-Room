#!/usr/bin/env bash
# Build the iOS app for the simulator. Pass a simulator name to also install and launch it.
set -euo pipefail
cd "$(dirname "$0")/.."
xcodegen generate --quiet
DERIVED="${FAMILY_ROOM_DERIVED:-$HOME/Library/Caches/com.familyroom.build}"
DEVICE="${1:-}"
xcodebuild -project FamilyRoom.xcodeproj -scheme FamilyRoom-iOS -configuration Debug \
  -destination "generic/platform=iOS Simulator" -derivedDataPath "$DERIVED" \
  CODE_SIGNING_ALLOWED=NO DEVELOPMENT_TEAM= -quiet build
APP="$DERIVED/Build/Products/Debug-iphonesimulator/Family Room.app"
if [[ -n "$DEVICE" ]]; then
  xcrun simctl boot "$DEVICE" 2>/dev/null || true
  xcrun simctl install "$DEVICE" "$APP"
  xcrun simctl launch "$DEVICE" com.familyroom.app
fi
echo "$APP"
