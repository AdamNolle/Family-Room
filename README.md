# Family Room

A private room for your family's photos and videos. Written in Swift for iPhone, iPad and Mac, synced through iCloud.

Photos gather into moments by day. The Hearth shows the year as a river you can scrub. Family Circle holds the people who appear in your photos, with faces grouped on your own device. Stories cuts short films from a moment's best clips, stitched on your device. Everyone you invite joins through an iCloud share.

## Requirements

- Xcode 16 or later on macOS 14 or later. Nothing else to install.
- An Apple Developer team for iCloud sync and sharing. Without one the app keeps its library on the device.

## Build

Open `FamilyRoom.xcodeproj` and run the `FamilyRoom-iOS` or `FamilyRoom-macOS` scheme, or use the Swift build tool:

```sh
swift Scripts/build.swift test              # unit tests
swift Scripts/build.swift ios "iPhone 16"   # build, install and launch in the simulator
swift Scripts/build.swift mac --run         # build and open the Mac app
swift Scripts/build.swift verify            # tests plus both builds, as CI runs them
```

To turn on iCloud, copy `Config/Local.example.xcconfig` to `Config/Local.xcconfig` and set your team ID.

In Debug builds, launch with `FAMILY_ROOM_DEMO=1` for a sample library. `FAMILY_ROOM_SCREEN=moment|circle|person|face|stories|compose|settings` opens a screen directly.

## Layout

| Folder | Contents |
|---|---|
| `Sources/FamilyRoom/Data` | Core Data model declared in code, `NSPersistentCloudKitContainer` with private and shared databases, `CKShare` invitations |
| `Sources/FamilyRoom/Media` | Import and thumbnails (ImageIO, AVFoundation), face grouping (Vision), clip ranking, film rendering |
| `Sources/FamilyRoom/Design` | Tokens from the Devices v3 design, the paper background, motion, shared components |
| `Sources/FamilyRoom/Screens` | Hearth, moment, Family Circle, person, Stories, composer, Settings |
| `Sources/FamilyRoom/Shell` | iPhone tab bar, iPad and Mac sidebar, open-foldable split layout |
| `Tests/UnitTests` | Catalog logic, the library store, face grouping and film rendering on real images, large-library timing |
| `Support` | Info.plist, entitlements, privacy manifest, app icon and its Swift source |

## Privacy

Photos stay in your family's iCloud. Faces, best clips and films are worked out on the device. No analytics, no third-party code.

## Releasing

See [docs/RELEASE.md](docs/RELEASE.md).
