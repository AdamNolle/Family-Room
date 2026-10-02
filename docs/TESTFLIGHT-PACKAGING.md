# Local-source beta packaging

The owner selected the newer local Family Room iPhone/iPad product for beta preparation. GitHub main did not contain this Rust/UniFFI Apple target. This branch imports the relevant existing local shared, Rust and Apple source; it does not include owner Android/Windows/Linux changes, local caches or CI changes.

The packaging fixes pin Rust iOS device/simulator objects to iOS 17, set app version 0.1.0 (1), declare iPad orientations, and add an opaque 1024px icon in the existing terracotta/ivory palette. The icon was generated for this beta, with the source image preserved in the preparation workspace.

Required-reason manifests are scoped to the app and its own FamilyCoreBindings framework. App-local preferences (selected room, motion, sounds, recovery acknowledgement) use CA92.1. App/selected file metadata used for vault/import/watch operation uses C617.1 and 3B52.1. Rust fs2 quota displays remaining space in Room Settings (85F4.1) and defers transfers if insufficient (E174.1). Runtime HTTP/lock timers measure elapsed in-app intervals (35F9.1); disk space and boot-time signals are not uploaded. These are required-reason API declarations only, with no fabricated tracking/collection or export classification.

References: https://developer.apple.com/documentation/bundleresources/describing-use-of-required-reason-api and its NSPrivacyAccessedAPIType reason reference.

The product uses separately implemented RustCrypto XChaCha20-Poly1305, sealed boxes, Ed25519 and rustls. The OS-only export answer used for 3DSeen/Habit Map does not apply. Export documentation/classification and actual ASC answers must be verified before uploading this beta. Independent cryptographic review remains a later release gate recorded in ACCESS_PROTOCOL.md; this branch makes no production approval claim.
