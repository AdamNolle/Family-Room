# Family Room

A private room for your family's photos and videos. Built in Swift for iPhone, iPad and Mac, synced through iCloud.

Photos gather into moments by day. The Hearth shows the year as a river you can scrub. Family Circle holds the people who appear in your photos, with faces grouped on your own device. Stories cuts short films from a moment's best clips, stitched on your device. Everyone you invite joins through an iCloud share.

## Requirements

- Xcode 16 or later, macOS 14 or later.
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`).
- An Apple Developer team for iCloud sync and sharing. Without one the app runs with a local-only library.

## Build

```sh
cp Config/Signing.example.xcconfig Config/Signing.xcconfig   # set DEVELOPMENT_TEAM
./script/build_and_run.sh          # macOS app
./script/build_ios.sh "iPhone 16"  # iOS Simulator, installs and launches
./script/verify.sh                 # unit tests + both builds
```

`swift build --build-system native` and `swift test --build-system native` work without Xcode project generation and exercise the same sources with a local-only library.

For a filled-in library during development, launch with `FAMILY_ROOM_DEMO=1` (Debug builds only). `FAMILY_ROOM_SCREEN=moment|circle|person|face|stories|compose|settings` opens a screen directly.

## How it is put together

| Area | Files |
|---|---|
| Data model and iCloud | `Sources/FamilyRoom/Data/` — Core Data model declared in code, `NSPersistentCloudKitContainer` with private and shared databases, `CKShare` invitations |
| Media | `Sources/FamilyRoom/Media/` — import and thumbnails (ImageIO, AVFoundation), face grouping (Vision), clip ranking, story rendering |
| Design | `Sources/FamilyRoom/Design/` — tokens from the "Devices v3" design, paper and sunlight background, shared components |
| Screens | `Sources/FamilyRoom/Screens/` — Hearth, moment, Family Circle, person, Stories, composer, Workshop settings |
| Shells | `Sources/FamilyRoom/Shell/` — iPhone tab bar, iPad and Mac sidebar, open-foldable split layout |

Rooms live in the owner's private CloudKit database and are shared with family through a `CKShare`; members see the Room from the shared database. Faces, clip choices and rendered films are computed on the device and stored in the Room. Author names on notes and additions come from a name you set in the Workshop and are kept in iCloud key-value storage.

## Releasing

See [docs/RELEASE.md](docs/RELEASE.md) for the App Store checklist, listing text and privacy answers.

## Privacy

Photos stay in your family's iCloud. Face grouping uses Vision feature prints computed on device and never contacts a service. No analytics, no third-party dependencies.
