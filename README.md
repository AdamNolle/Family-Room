# Family Room

A private home for shared photos and videos, built with native interfaces and a shared Rust library.

This repository now contains a working development foundation, replacing the sample-only Apple entry point. It is **not the complete production release** described in the [product specification](docs/PRODUCT.md). The [feature register](docs/FEATURE_REGISTER.md) records verified behavior, partial implementations, and remaining work.

To invite an album-only viewer, open an album and choose **Share this album**. Publish its current contents, then create an invitation with the recipient’s device code. The published library has its own encryption key and storage setup; connect a gateway/source or export its encrypted Room package. Publish again after album changes.

## Run on macOS

Install Xcode, Rust/rustup, and XcodeGen for the iOS project. On this Apple Silicon development machine:

```sh
./script/build_and_run.sh --verify
```

The Codex **Run** action uses the same script. It builds the Rust core and SwiftUI executable, signs the development bundle in a local cache outside File Provider folders, links it from `dist/FamilyRoom.app`, and launches it through Launch Services. Local development uses ad hoc signing without accessing signing-certificate keys. Set FAMILY_ROOM_SIGN_IDENTITY to opt into a certificate explicitly. Distribution signing and notarization remain open. The launcher restarts only the matching macOS bundle, preserving simulator apps.

macOS Debug builds use a persistent, separate development library under `~/Library/Application Support/FamilyRoom/Development/Vault`. Its recovery key is in an owner-only file under `Development/DevelopmentKeys`, outside exported backups. This prevents Keychain prompts during rebuilds. A local key file has less protection than Keychain and is intended for development data. Release builds and iOS still use Keychain; the existing `FamilyRoom/Vault` library remains intact. Showing the recovery key uses the already unlocked session rather than making another Keychain request.

Create a Room, import files with **Add**, and open **Room settings** to configure storage or export a recovery backup. The app starts with an empty library. No sample family photos or members are presented as real content.

## What works

- Persistent, encrypted local catalog and originals; distinct Room, Member, Person, Moment, Album, and Story identities.
- Recipient-sealed device invitations, owner/admin/contributor/viewer grants, role changes and device revocation with new key epochs; portable encrypted Room exchange.
- Real imports, exact duplicate detection, captions, dates, favorites, People tags, albums, user-featured Moments, comments, and reversible deletion.
- Folder watches and Apple Photos automatic import into an explicit Room while the app runs; pause and historical-import controls.
- Local source copies and resumable jobs, quota/budget checks, preferred destinations, verified-copy reporting, and encrypted backup/restore.
- Recovery export for offline work preserved after access changes, including private film drafts and verified available originals. The exported ZIP is unencrypted; keep it private.
- Cloud adapter code for Google Drive, OneDrive, WebDAV/Nextcloud, plus a self-hosted ciphertext gateway. Cloud credentials currently require manual configuration; live account validation and interactive OAuth are still outstanding.
- SwiftUI film drafts, layers, trim/speed, opacity/exposure/saturation, titles, fades, volume automation, chapters, autosave, undo/redo, and an actual MP4 renderer.
- Native SwiftUI Apple app, Kotlin/Compose Android client, C#/WinUI Windows client, and Rust/GTK/libadwaita Linux client sources. Feature parity and all-platform runtime validation are still outstanding.

## Build and verify

```sh
./script/verify.sh          # Rust format/lint/tests, bindings, native Swift tests
./script/build_ios.sh      # Rust XCFramework + XcodeGen + arm64 simulator build
./script/build_android.sh  # NDK arm64/x86_64 + bindings + debug APK
# Windows host: ./script/build_windows.ps1
cargo check --manifest-path platforms/linux/Cargo.toml
```

Android requires Java 17, Android SDK 35, and NDK 27.2.12479018. Linux requires GTK 4, libadwaita, and Secret Service. Windows requires the .NET 8 SDK, Windows App SDK tools, and an appropriate Windows build host. See [validation](docs/VALIDATION.md) for results and limits.

## Release gates still open

Peer discovery/transfers, the iCloud adapter, OS background scheduling, interactive provider sign-in, complete restoration and cross-platform runtime certification, recognition/search intelligence, and the complete advanced creative studio remain unfinished. Album-only sharing uses explicitly published, independently encrypted viewer libraries; continuous restricted-album collaboration remains unfinished. Device revocation takes effect after the signed access update reaches storage and devices; previously downloaded memories remain readable. Ownership transfer, multi-device member profiles and independent cryptographic review remain open.

[Product and milestones](docs/PRODUCT.md) · [Feature register](docs/FEATURE_REGISTER.md) · [Architecture and protocol](docs/ARCHITECTURE.md) · [UI direction](docs/UI_DESIGN.md)
