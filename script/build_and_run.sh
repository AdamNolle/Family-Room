#!/usr/bin/env bash
# Generate the Xcode project, build the macOS app, and launch it.
# With no DEVELOPMENT_TEAM in Config/Signing.xcconfig the app is ad-hoc signed without iCloud entitlements
# and keeps its library on this Mac.
set -euo pipefail
cd "$(dirname "$0")/.."
xcodegen generate --quiet
DERIVED="${FAMILY_ROOM_DERIVED:-$HOME/Library/Caches/com.familyroom.build}"
TEAM="$(sed -n 's/^DEVELOPMENT_TEAM *= *//p' Config/Signing.xcconfig 2>/dev/null | tr -d ' ')"
EXTRA=()
if [[ -z "$TEAM" ]]; then EXTRA=(CODE_SIGN_IDENTITY=- "CODE_SIGN_ENTITLEMENTS=" CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM=); fi
xcodebuild -project FamilyRoom.xcodeproj -scheme FamilyRoom-macOS -configuration Debug \
  -derivedDataPath "$DERIVED" -quiet "${EXTRA[@]}" build
APP="$DERIVED/Build/Products/Debug/Family Room.app"
case "${1:-run}" in
  --build-only) echo "$APP" ;;
  run) open -n "$APP" ;;
  *) echo "usage: $0 [run|--build-only]" >&2; exit 2 ;;
esac
