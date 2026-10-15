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
- Review invitations before joining. Pending invites survive restart in the recipient’s encrypted local catalog; accepting joins the Room, while declining removes the local invite without notifying its sender.
- Real imports, exact duplicate detection, on-device similar-photo suggestions for review (never auto-deleted), captions, dates, favorites, People tags, ordered albums with notes and chosen covers, user-featured Moments, comments, per-member Room likes, and reversible deletion.
- Home resurfaces earlier photos and videos captured on this local calendar date. February 29 memories appear on February 28 in non-leap years.
- **Ask your videos** indexes local speech, sampled frames, visible text and available sound/scene clues on the current device. Search returns timestamped evidence and opens the original at the matching moment; exact phrases are strict transcript/OCR matches, while inferred sound and scene labels are suggestions. Remote-only originals are included only after opting in.
- People offers explicit on-device face-crop review on Apple and Android. It scans only photos already saved to the Room on that device, then waits for a member to choose or create a Person before adding a whole-photo tag. Crops stay temporary; the app does not guess identities, retain face templates, or group people across photos. Windows/Linux face review remains open.
- Automatic sharing uses Apple Photos on Apple platforms and explicit local-folder watches on Android, Apple desktop, Linux, and Windows. Each folder rule targets one Room and supports opt-in historical import, extension exclusions, and pause/resume. Android uses the system folder picker and periodic WorkManager scans; Linux and Windows scan every 15 seconds while Family Room is open. Background timing follows each operating system’s scheduling policy.
- Local source copies and resumable jobs, quota/budget checks, preferred destinations, an explicit same-Room overflow source, verified-copy reporting, and encrypted backup/restore. Room settings can reclaim a chosen amount of local space or set an opt-in 2 GB, 5 GB, or 10 GB device cache limit. After Room sync, an original is removed only after another copy verifies; pinned, damaged, and sole copies stay on the device, with deleted, non-favorite, older memories considered first. Per-source unmetered-only settings queue original-media uploads and downloads while encrypted catalog updates continue.
- Recovery export for offline work preserved after access changes, including private film drafts and verified available originals. The exported ZIP is unencrypted; keep it private.
- Cloud adapter code for Google Drive, OneDrive, WebDAV/Nextcloud, plus a self-hosted ciphertext gateway. Cloud credentials currently require manual configuration; live account validation and interactive OAuth are still outstanding.
- Apple and Android Theater support ordered playlists of published Room films, plus a short disc-insertion animation followed by native playback. Film drafts remain private, and reduced-motion settings skip the animation.

- SwiftUI film drafts, layers, trim/speed, opacity/exposure/saturation, titles, fades, volume automation, chapters, autosave, undo/redo, and an actual MP4 renderer.
- Native SwiftUI Apple app, Kotlin/Compose Android client, C#/WinUI Windows client, and Rust/GTK/libadwaita Linux client sources. All four client families expose album and playlist organization. Windows and Linux include native continuous playlist players that prepare one film at a time; Windows compilation and Linux runtime playback still need host-platform validation. Android includes a native layered film studio and on-device MP4 rendering/playback/export/publication. Feature parity and all-platform runtime validation are still outstanding.
- **Ask your videos** searches timestamped speech, on-screen words, and visual clues in an encrypted, device-local index; Apple also indexes possible sound events. Apple uses Vision, on-device Speech, and Sound Analysis; Android uses ML Kit and optional verified English Vosk speech recognition; Windows/Linux use FFmpeg and Tesseract, with optional local whisper.cpp transcription. A loopback-only local vision service can add timestamped descriptions of visible objects and actions on Windows/Linux. A small shared concept vocabulary improves suggestions for common paraphrases such as *audience applauding* → *crowd clapping*; these remain suggestions. Exact matches come only from ordered transcript or OCR wording. Results link to their timestamp. The Rust core builds a bounded inverted word index from timestamped transcript, OCR, sound, and visual evidence; token postings stay in memory while encrypted observations remain in the local catalog. Indexes do not sync between Room members, and remote-only videos are downloaded only after opting in. Explicit follow-up questions reuse the previous question’s searchable terms on this device; quoted phrases start a fresh exact lookup. General embedding-based semantic search, speaker identity, non-Apple sound-event classification, model setup UX, broad family-video accuracy, and large-library performance remain open.

Windows/Linux indexing uses the Rust `family-video-indexer` helper with FFmpeg (`ffmpeg` and `ffprobe`) plus Tesseract for on-screen text. To index speech, install whisper.cpp and a local GGML model, then set `FAMILY_ROOM_WHISPER_MODEL`. To add visual object/action descriptions, run a local OpenAI-compatible vision service bound to loopback and set `FAMILY_ROOM_VISION_URL`; an optional model name can be set with `FAMILY_ROOM_VISION_MODEL_NAME`. Tool paths and OCR language can be configured with `FAMILY_ROOM_FFMPEG`, `FAMILY_ROOM_FFPROBE`, `FAMILY_ROOM_TESSERACT`, `FAMILY_ROOM_TESSERACT_LANG`, and `FAMILY_ROOM_WHISPER_CLI`. The helper processes video on this device and stores only timestamped text evidence in the encrypted local catalog. See [video search and indexing](docs/VIDEO_SEARCH.md) for the pipeline, result guarantees, and current limits.

For Nextcloud, choose **WebDAV / Nextcloud** in Room settings, enter a dedicated Room folder URL under `/remote.php/dav/files/<username>/`, and use a [Nextcloud app password](https://docs.nextcloud.com/server/stable/developer_manual/client_apis/WebDAV/basic.html). The app reads actual [WebDAV quota properties](https://www.rfc-editor.org/rfc/rfc4331.html) when the server reports them. Servers without those properties display an unknown quota. A live Nextcloud account has not yet been validated.

## Build and verify

```sh
./script/verify.sh          # Rust format/lint/tests, bindings, native Swift tests
./script/build_ios.sh      # Rust XCFramework + XcodeGen + arm64 simulator build
./script/build_android.sh  # NDK arm64/x86_64, JVM tests, debug APK, and QA test APK compilation
# Windows host: ./script/build_windows.ps1
cargo check --manifest-path platforms/linux/Cargo.toml
```

Android requires Java 17, Android SDK 36, and NDK 27.2.12479018. Linux requires GTK 4, libadwaita, and Secret Service. Windows requires the .NET 8 SDK, Windows App SDK tools, and an appropriate Windows build host. See [validation](docs/VALIDATION.md) for results and limits.

## Release gates still open

Peer discovery/transfers, the iCloud adapter, OS background scheduling, interactive provider sign-in, complete restoration and cross-platform runtime certification, recognition/search intelligence, and the complete advanced creative studio remain unfinished. Album-only sharing uses explicitly published, independently encrypted viewer libraries; continuous restricted-album collaboration remains unfinished. Device revocation and owner succession take effect after a signed access update reaches storage and devices; previously downloaded memories remain readable. Members can leave locally on one device; this forgets its active Room keys and stops local sync but does not change the shared signed roster. An owner must separately revoke the device for Room-wide removal. Multi-device member profiles and independent cryptographic review remain open.

[Product and milestones](docs/PRODUCT.md) · [Feature register](docs/FEATURE_REGISTER.md) · [Architecture and protocol](docs/ARCHITECTURE.md) · [UI direction](docs/UI_DESIGN.md)

Android compiler outputs use an isolated local cache outside synced checkouts. `FAMILY_ROOM_ANDROID_BUILD_DIR` overrides that location; `./gradlew --quiet printAppBuildDirectory` reports it. The build script copies the debug APK to `dist/android/FamilyRoom-debug.apk`. Instrumentation uses a separate `com.familyroom.app.qa` application and `connectedQaAndroidTest`, preserving the regular development library.
