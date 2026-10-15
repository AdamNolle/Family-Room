# Native UI review

## Final brown, taupe and pale-blue palette

These are captures of the running native apps, not HTML mockups. The final direction uses warm white, taupe, orange-brown, apricot and pale blue, restrained glass, and black/white photo frames. Photo colors are preserved. The canvas has no cork texture and the composition is specific to Family Room.

- macOS: [Home](home-seasonal-macos.png), [Library](library-seasonal-macos.png), [Albums](albums-seasonal-macos.png), [People](people-seasonal-macos.png), [Theater](theater-seasonal-macos.png).
- Apple mobile: [iPhone Home](home-seasonal-iphone.jpg), [iPad Home](home-seasonal-ipad.jpg).
- Android phone: [Home](home-seasonal-android-phone.png), [Theater](theater-seasonal-android-phone.png).
- Android video finder: [timestamped evidence search and honest empty result](video-finder-android.png), captured from the native API 35 phone QA app after search and keyboard dismissal. The separate test Room has zero indexed videos; this capture verifies layout and state clarity, while a separate instrumentation test exercised real MP4 frame analysis.
- Android tablet: [Home](home-seasonal-android-tablet.png), [Theater](theater-seasonal-android-tablet.png).
- Android film: [Published film entering the player](theater-disc-android.png). This is a native full-screen capture from the QA Room. Its solid orange artwork is the actual deliberately synthetic test movie frame, not a family photo or a bundled marketing image.

The Mac Design review Room contains five publicly accessible Nolle Studios portfolio photographs imported through the real file picker for this local review. These photographs are not bundled app assets, demo-library seeds or sent to another member or provider. Autumn collection is a real locally created album with an assigned original. Review Person is a manually created local Person with no confirmed tags; it does not imply working face recognition or membership.

macOS build, ad-hoc signature and launch passed without a Keychain prompt. Real search/clear, imports, detail, album assignment and Person creation were reviewed. The iPhone navigation test passed after structural UI changes, followed by a final-palette install and capture. The iPad native navigation test passed on the final palette (one test, zero failures). The final Android navigation test passed on both phone and tablet (one test, zero failures per device). Apple mobile captures show genuine empty Room states. Android captures use the separate QA catalog; the final tablet capture is also an empty Room. Synthetic imported media appears in the earlier studio review below. These screenshots do not establish full cross-platform feature parity, accessibility, dark mode, background transfers or physical-device performance.

## Earlier Android film workspace review

The following studio captures predate the final gallery palette and document functional workspace layout only.

Captured on 2026-10-02 from the actual Compose application on an API 35 arm64 Pixel Tablet emulator (2560 × 1600 pixels, 1280 × 800 dp). The QA application uses the real JNI catalog and Keystore in a separate test Room. The ochre image is a deliberately synthetic PNG imported through the shared core, not family photography or a placeholder inserted into the view model.

- [Selected inspector](film-inspector-android.png): selected imported clip, independently scrolled inspector, visible Render controls and docked timeline.
- [Saved split project](film-studio-android.png): two clips produced by the shared split command, saved private title, visible history controls and selected-clip inspector after keyboard dismissal.

Native tests exercise adding, splitting, undo/redo, saving, reopening after Activity recreation, actual MP4 rendering and explicit publication to this local test Room. A separate renderer test decodes exported video pixels and audio samples. The captures show a still original preview before rendering; they do not prove video playback, external document-provider export, screen-reader navigation, large-text behavior or physical-device performance. No external provider or other member is connected to the test Room.
