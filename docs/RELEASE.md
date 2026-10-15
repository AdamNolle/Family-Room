# Releasing Family Room

Everything below the line "Needs your Apple Developer team" requires an account; everything above it is done in the repository.

## Ready in the repository

- App icon for iOS and macOS (`Support/Assets.xcassets`, regenerate with `swift Support/IconSource/render_icon.swift <dir>`).
- Privacy manifest (`Support/PrivacyInfo.xcprivacy`): no tracking, no collected data, reasons for UserDefaults and file timestamps.
- Entitlements per platform (`Support/FamilyRoom-iOS.entitlements`, `Support/FamilyRoom-macOS.entitlements`) for CloudKit, push and iCloud key-value storage. The Mac build is sandboxed with hardened runtime.
- `ITSAppUsesNonExemptEncryption` is `false`: the app uses only Apple's own encryption through iCloud.
- Debug-only sample data and screen hooks are compiled out of Release builds.

## Needs your Apple Developer team

1. Copy `Config/Signing.example.xcconfig` to `Config/Signing.xcconfig` and set `DEVELOPMENT_TEAM`.
2. In Certificates, Identifiers & Profiles, register `com.familyroom.app` with iCloud (CloudKit) and Push Notifications, and create the container `iCloud.com.familyroom.app`.
3. Run a Debug build on two devices signed in to different Apple Accounts. Create a Room on one, invite the other from Settings, accept, and confirm photos, notes and likes appear on both. Faces and films are computed on each device and arrive as Room data.
4. In the CloudKit Console, deploy the development schema to production once step 3 passes. Release builds use the production environment.
5. Archive each platform in Xcode (Product › Archive) and upload to App Store Connect. Automatic signing switches the push entitlement to production.
6. Fill in the listing below and submit for review.

## App Store listing

**Name:** Family Room

**Subtitle:** Your family's photos, together

**Description:**
Family Room is a private place for the photos and videos your family takes.

Add pictures and they gather into moments by day. Scroll the year on the Hearth, scrub through it on the river of time, and open any day to see every photo laid out like prints on a table, with notes from everyone who was there.

Invite the people you love through iCloud. Everyone in the Room sees new photos as they arrive, adds their own, and leaves a note or a heart.

Family Circle groups faces on your own device so you can name the people who appear in your photos and see every moment they were part of. Nothing is sent to a server to do it.

Make a short film from any day. Family Room picks the strongest clips, you choose which to keep, and it stitches them into a movie right on your iPhone, iPad or Mac.

Photos stay in your family's iCloud. No ads, no accounts to create, no tracking.

**Keywords:** family,photos,album,shared,iCloud,memories,video,moments,faces,film

**Category:** Photo & Video

## App privacy answers

- Data collection: **No, we do not collect data from this app.** Photos, videos, notes and face data are stored in the user's own iCloud and shared only with people they invite; the developer never receives them.
- Tracking: **No.**
