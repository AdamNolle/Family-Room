import AVFoundation
import FamilyCoreBindings
import ImageIO
import UniformTypeIdentifiers
import XCTest

@testable import FamilyRoom

final class LibraryTests: XCTestCase {
  func testMomentFilterUsesRoomMomentMembership() {
    let moments = [
      LibraryMoment(
        id: "summer",
        roomId: "room",
        title: "Summer at the lake",
        start: 100,
        end: 200,
        assetIds: ["photo-1", "video-1"],
        featured: false
      )
    ]

    XCTAssertTrue(
      libraryMomentMatchesAsset(assetId: "outside", selectedMomentId: nil, moments: moments))
    XCTAssertTrue(
      libraryMomentMatchesAsset(assetId: "photo-1", selectedMomentId: "summer", moments: moments))
    XCTAssertFalse(
      libraryMomentMatchesAsset(assetId: "outside", selectedMomentId: "summer", moments: moments))
    XCTAssertFalse(
      libraryMomentMatchesAsset(assetId: "photo-1", selectedMomentId: "missing", moments: moments))
  }

  func testCaptureDateCorrectionChangesTheDayButPreservesLocalTimeAndSpacing() {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(secondsFromGMT: -5 * 60 * 60)!
    let original = calendar.date(
      from: DateComponents(year: 2020, month: 2, day: 3, hour: 14, minute: 27, second: 31))!
    let target = calendar.date(from: DateComponents(year: 2024, month: 11, day: 5, hour: 0))!
    let first = Int64(original.timeIntervalSince1970)
    let second = first + 3_600

    let correctedFirst = correctedCaptureTimestamp(first, to: target, calendar: calendar)
    let correctedSecond = correctedCaptureTimestamp(second, to: target, calendar: calendar)
    let correctedDate = Date(timeIntervalSince1970: TimeInterval(correctedFirst))

    XCTAssertEqual(calendar.component(.year, from: correctedDate), 2024)
    XCTAssertEqual(calendar.component(.month, from: correctedDate), 11)
    XCTAssertEqual(calendar.component(.day, from: correctedDate), 5)
    XCTAssertEqual(calendar.component(.hour, from: correctedDate), 14)
    XCTAssertEqual(calendar.component(.minute, from: correctedDate), 27)
    XCTAssertEqual(calendar.component(.second, from: correctedDate), 31)
    XCTAssertEqual(correctedSecond - correctedFirst, 3_600)
  }

  func testOfflinePinFieldDecodesInCurrentAndLegacyCatalogs() throws {
    var asset = LibraryAsset(
      id: "photo", roomId: "room", filename: "photo.jpg", kind: "photo", sha256: "hash", size: 12,
      capturedAt: 0, contributor: "member", favorite: false, caption: "", deleted: false,
      people: [], local: true, verifiedCopies: 1, availability: "Saved on this device")
    asset.pinnedOffline = true
    let currentData = try LibraryCoding.encoder.encode(asset)
    XCTAssertEqual(
      try LibraryCoding.decoder.decode(LibraryAsset.self, from: currentData).pinnedOffline, true)

    var legacyPayload = try XCTUnwrap(
      JSONSerialization.jsonObject(with: currentData) as? [String: Any])
    legacyPayload.removeValue(forKey: "pinned_offline")
    let legacyData = try JSONSerialization.data(withJSONObject: legacyPayload)
    XCTAssertNil(
      try LibraryCoding.decoder.decode(LibraryAsset.self, from: legacyData).pinnedOffline)
  }

  func testDeletionSelectionRequiresOwnerAdminOrOnlyTheContributorsOwnMemories() {
    XCTAssertTrue(
      canChangeDeletionForSelection(
        role: "contributor", currentDeviceID: "device-a", contributors: ["device-a", "device-a"]
      ))
    XCTAssertFalse(
      canChangeDeletionForSelection(
        role: "contributor", currentDeviceID: "device-a", contributors: ["device-a", "device-b"]
      ))
    XCTAssertTrue(
      canChangeDeletionForSelection(
        role: "admin", currentDeviceID: "device-a", contributors: ["device-b"]
      ))
    XCTAssertFalse(
      canChangeDeletionForSelection(
        role: "viewer", currentDeviceID: "device-a", contributors: ["device-a"]
      ))
    XCTAssertFalse(
      canChangeDeletionForSelection(
        role: "owner", currentDeviceID: "device-a", contributors: []
      ))
  }

  func testVideoFinderDecodesCertaintyAndMatchedTimeBounds() throws {
    let json = """
      {
        "answer": "One exact match.",
        "matches": [{
          "room_id": "room",
          "asset_id": "movie",
          "start_ms": 12000,
          "end_ms": 14500,
          "score": 3,
          "match_kind": "suggestion",
          "evidence": [{
            "modality": "visual_label",
            "start_ms": 12000,
            "end_ms": 12000,
            "text": "red bicycle",
            "confidence_percent": 55
          }]
        }],
        "coverage": {"videos": 4, "indexed": 3, "audio_indexed": 2, "frames_indexed": 3}
      }
      """
    let response = try LibraryCoding.decoder.decode(VideoFinderResponse.self, from: Data(json.utf8))
    let match = try XCTUnwrap(response.matches.first)

    XCTAssertEqual(match.matchKind, "suggestion")
    XCTAssertEqual(match.startMs, 12_000)
    XCTAssertEqual(match.endMs, 14_500)
    XCTAssertEqual(match.evidence.first?.modality, "visual_label")
  }

  func testVideoFinderLabelsSoundClassifierEvidenceAsAPossibility() throws {
    let json =
      #"{"answer":"No exact match.","matches":[{"room_id":"room","asset_id":"movie","start_ms":18000,"end_ms":21000,"score":91,"match_kind":"suggestion","evidence":[{"modality":"sound_event","start_ms":18000,"end_ms":21000,"text":"applause","confidence_percent":91}]}],"coverage":{"videos":1,"indexed":1,"audio_indexed":0,"sound_events_indexed":1,"frames_indexed":0}}"#
    let response = try LibraryCoding.decoder.decode(VideoFinderResponse.self, from: Data(json.utf8))
    let match = try XCTUnwrap(response.matches.first)

    XCTAssertEqual(match.matchKind, "suggestion")
    XCTAssertEqual(match.evidence.first?.label, "Possible sound")
    XCTAssertEqual(response.coverage.soundEventsIndexed, 1)
  }

  func testNativeCatalogDecodesAlbumsAndPlaylistsWithoutChangingOriginals() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let core = try FamilyCore.open(directory: directory.path, recoveryKey: newRecoveryKey())
    func command(_ request: [String: Any]) throws -> [String: Any] {
      let payload = try JSONSerialization.data(withJSONObject: request)
      let response = try core.command(request: String(decoding: payload, as: UTF8.self))
      return try XCTUnwrap(
        JSONSerialization.jsonObject(with: Data(response.utf8)) as? [String: Any])
    }
    let roomId = try XCTUnwrap(
      command(["action": "create_room", "name": "Collections"])["id"] as? String)
    let photo = directory.appendingPathComponent("memory.png")
    try Data(
      base64Encoded:
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aJ1sAAAAASUVORK5CYII="
    )!.write(to: photo)
    let assetId = try XCTUnwrap(
      command(["action": "import", "room_id": roomId, "path": photo.path])["id"] as? String)
    let albumId = try XCTUnwrap(
      command([
        "action": "save_album", "room_id": roomId, "title": "Autumn",
        "caption": "The first cool afternoon", "asset_ids": [assetId], "cover_asset_id": assetId,
      ])["id"] as? String)
    let playlistId = try XCTUnwrap(
      command([
        "action": "save_playlist", "room_id": roomId, "title": "Friday films", "story_ids": [],
      ])["id"] as? String)
    _ = try command(["action": "set_like", "room_id": roomId, "asset_id": assetId, "liked": true])
    let response = try command(["action": "snapshot"])
    let snapshot = try LibraryCoding.decoder.decode(
      LibrarySnapshot.self, from: JSONSerialization.data(withJSONObject: response))
    let room = try XCTUnwrap(snapshot.rooms.first { $0.id == roomId })
    XCTAssertEqual(room.albums.first { $0.id == albumId }?.assetIds, [assetId])
    XCTAssertEqual(room.albums.first { $0.id == albumId }?.captionText, "The first cool afternoon")
    XCTAssertEqual(room.albums.first { $0.id == albumId }?.coverAssetId, assetId)
    XCTAssertEqual(room.curatedPlaylists.first { $0.id == playlistId }?.title, "Friday films")
    XCTAssertEqual(room.assets.first { $0.id == assetId }?.deleted, false)
    XCTAssertEqual(room.likeCount(for: assetId), 1)
    XCTAssertTrue(room.hasLiked(assetId, deviceId: snapshot.deviceId))
    XCTAssertEqual(room.films(in: try XCTUnwrap(room.curatedPlaylists.first)), [])
  }
  func testSharedSplitThroughNativeBoundaryMatchesPortableAutomation() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let core = try FamilyCore.open(directory: directory.path, recoveryKey: newRecoveryKey())
    func command(_ request: [String: Any]) throws -> [String: Any] {
      let response = try core.command(
        request: String(decoding: JSONSerialization.data(withJSONObject: request), as: UTF8.self))
      return try XCTUnwrap(
        JSONSerialization.jsonObject(with: Data(response.utf8)) as? [String: Any])
    }
    let room = try XCTUnwrap(
      command(["action": "create_room", "name": "Timeline boundary"])["id"] as? String)
    let file = directory.appendingPathComponent("fixture.png")
    try Data(
      base64Encoded:
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aJ1sAAAAASUVORK5CYII="
    )!.write(to: file)
    let asset = try XCTUnwrap(
      command(["action": "import", "room_id": room, "path": file.path])["id"] as? String)
    let created = try command(["action": "create_story", "room_id": room])
    var project = try LibraryCoding.decoder.decode(
      FilmProject.self, from: JSONSerialization.data(withJSONObject: created["project"] as Any))
    let clip = FilmClip(
      assetId: asset, start: 2, trimIn: 1, duration: 8, speed: 2, fadeIn: 3, fadeOut: 3,
      opacityKeyframes: [FilmKeyframe(time: 0, value: 0), FilmKeyframe(time: 8, value: 1)],
      volumeKeyframes: [FilmKeyframe(time: 0, value: 0.2), FilmKeyframe(time: 8, value: 0.8)])
    project.clips = [clip]
    let value = try JSONSerialization.jsonObject(with: LibraryCoding.encoder.encode(project))
    let response = try command([
      "action": "split_story_clip", "project": value, "clip_id": clip.id, "source_time": 4,
    ])
    let shared = try LibraryCoding.decoder.decode(
      FilmProject.self, from: JSONSerialization.data(withJSONObject: response["project"] as Any))
    let expected = try XCTUnwrap(clip.split(at: 4))
    XCTAssertTrue(shared.isPrivateDraft)
    XCTAssertEqual(shared.formatVersion, 1)
    XCTAssertEqual(shared.clips.count, 2)
    for (actual, local) in zip(shared.clips, [expected.left, expected.right]) {
      XCTAssertEqual(actual.start, local.start)
      XCTAssertEqual(actual.trimIn, local.trimIn)
      for step in 0...50 {
        let time = actual.duration / actual.speed * Double(step) / 50
        XCTAssertEqual(actual.audioGain(at: time), local.audioGain(at: time), accuracy: 0.000001)
        XCTAssertEqual(actual.opacity(at: time), local.opacity(at: time), accuracy: 0.000001)
      }
    }
    let snapshot = try command(["action": "snapshot"])
    let typed = try LibraryCoding.decoder.decode(
      LibrarySnapshot.self, from: JSONSerialization.data(withJSONObject: snapshot))
    XCTAssertEqual(
      typed.rooms.first?.stories.first?.clips.count, 0,
      "A derived split must not save or publish itself.")
  }
  func testNativeBoundaryReadsRealEncryptedCatalog() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let core = try FamilyCore.open(directory: directory.path, recoveryKey: newRecoveryKey())
    let created = try core.command(request: "{\"action\":\"create_room\",\"name\":\"Friends\"}")
    let createdValue = try XCTUnwrap(
      JSONSerialization.jsonObject(with: Data(created.utf8)) as? [String: Any])
    let roomId = try XCTUnwrap(createdValue["id"] as? String)
    let policy = try JSONSerialization.data(withJSONObject: [
      "action": "set_device_cache_limit", "room_id": roomId, "max_bytes": 2_147_483_648,
    ])
    _ = try core.command(request: String(decoding: policy, as: UTF8.self))
    let response = try core.command(request: "{\"action\":\"snapshot\"}")
    let snapshot = try LibraryCoding.decoder.decode(LibrarySnapshot.self, from: Data(response.utf8))
    XCTAssertEqual(snapshot.rooms.first?.name, "Friends")
        XCTAssertEqual(snapshot.rooms.first?.role, "owner")
        XCTAssertEqual(snapshot.rooms.first?.deviceCacheLimitBytes, 2_147_483_648)
        XCTAssertEqual(snapshot.rooms.first?.onThisDayAssetIds, [])
        XCTAssertEqual(snapshot.rooms.first?.assets.count, 0)
  }
  func testTimelineInterpolationAndSpeed() {
    let clip = FilmClip(assetId: "one", start: 2, duration: 8, speed: 2)
    XCTAssertEqual(clip.end, 6)
    XCTAssertEqual(
      clip.value(
        at: 1, keyframes: [FilmKeyframe(time: 0, value: 0), FilmKeyframe(time: 2, value: 1)],
        fallback: 1), 0.5)
    XCTAssertEqual(clip.value(at: 3, keyframes: [], fallback: 0.7), 0.7)
  }
  func testPortableProjectRoundTrip() throws {
    let project = FilmProject(
      roomId: "room",
      clips: [
        FilmClip(
          assetId: "photo", track: 2, exposure: 0.5,
          opacityKeyframes: [FilmKeyframe(time: 0, value: 0)])
      ])
    let data = try LibraryCoding.encoder.encode(project)
    XCTAssertTrue(String(decoding: data, as: UTF8.self).contains("opacity_keyframes"))
    XCTAssertEqual(try LibraryCoding.decoder.decode(FilmProject.self, from: data), project)
  }
  func testPhotoFilmExportIsPlayableAndKeepsItsDuration() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }
    let photo = root.appendingPathComponent("red.png")
    let context = try XCTUnwrap(
      CGContext(
        data: nil, width: 128, height: 128, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
    context.setFillColor(CGColor(red: 1, green: 0, blue: 0, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: 128, height: 128))
    let image = try XCTUnwrap(context.makeImage())
    let destination = try XCTUnwrap(
      CGImageDestinationCreateWithURL(photo as CFURL, UTType.png.identifier as CFString, 1, nil))
    CGImageDestinationAddImage(destination, image, nil)
    XCTAssertTrue(CGImageDestinationFinalize(destination))
    let asset = LibraryAsset(
      id: "photo", roomId: "room", filename: "red.png", kind: "photo", sha256: "", size: 0,
      capturedAt: 0, contributor: "", favorite: false, caption: "", deleted: false, people: [],
      local: true, verifiedCopies: 1, availability: "Saved on this device")
    let project = FilmProject(
      roomId: "room", width: 128, height: 128, clips: [FilmClip(assetId: "photo", duration: 0.4)])
    let output = root.appendingPathComponent("film.mp4")
    try await FilmRenderer.render(
      project, assets: [asset], urls: [asset.id: photo], destination: output)
    let rendered = AVURLAsset(url: output)
    let tracks = try await rendered.loadTracks(withMediaType: .video)
    XCTAssertEqual(tracks.count, 1)
    let duration = try await rendered.load(.duration)
    XCTAssertEqual(duration.seconds, 0.4, accuracy: 0.06)
    let generator = AVAssetImageGenerator(asset: rendered)
    let frame = try await generator.image(at: CMTime(seconds: 0.1, preferredTimescale: 600))
    XCTAssertEqual(frame.image.width, 128)
    let pixelContext = try XCTUnwrap(
      CGContext(
        data: nil, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
    pixelContext.draw(frame.image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
    let pixel = try XCTUnwrap(pixelContext.data).assumingMemoryBound(to: UInt8.self)
    XCTAssertGreaterThan(pixel[0], 200, "The exported movie must contain the red source image.")
    XCTAssertLessThan(pixel[1], 30)
    XCTAssertLessThan(pixel[2], 30)
  }
  func testLayeredFilmExportPreservesTimingAndAudibleAudio() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }
    func photo(_ name: String, red: CGFloat, blue: CGFloat) throws -> URL {
      let url = root.appendingPathComponent(name)
      let context = try XCTUnwrap(
        CGContext(
          data: nil, width: 128, height: 128, bitsPerComponent: 8,
          bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(),
          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
      context.setFillColor(CGColor(red: red, green: 0, blue: blue, alpha: 1))
      context.fill(CGRect(x: 0, y: 0, width: 128, height: 128))
      let destination = try XCTUnwrap(
        CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil))
      CGImageDestinationAddImage(destination, try XCTUnwrap(context.makeImage()), nil)
      XCTAssertTrue(CGImageDestinationFinalize(destination))
      return url
    }
    let red = try photo("red.png", red: 1, blue: 0)
    let blue = try photo("blue.png", red: 0, blue: 1)
    let audio = root.appendingPathComponent("tone.wav")
    var pcm = Data()
    func little<T: FixedWidthInteger>(_ value: T) -> Data {
      var number = value.littleEndian
      return withUnsafeBytes(of: &number) { Data($0) }
    }
    for i in 0..<8000 { pcm.append(little(Int16(sin(Double(i) * 2 * .pi * 440 / 16000) * 16000))) }
    var wave = Data("RIFF".utf8)
    wave.append(little(UInt32(36 + pcm.count)))
    wave.append(Data("WAVEfmt ".utf8))
    wave.append(little(UInt32(16)))
    wave.append(little(UInt16(1)))
    wave.append(little(UInt16(1)))
    wave.append(little(UInt32(16000)))
    wave.append(little(UInt32(32000)))
    wave.append(little(UInt16(2)))
    wave.append(little(UInt16(16)))
    wave.append(Data("data".utf8))
    wave.append(little(UInt32(pcm.count)))
    wave.append(pcm)
    try wave.write(to: audio)
    func asset(_ id: String, _ filename: String, _ kind: String) -> LibraryAsset {
      LibraryAsset(
        id: id, roomId: "room", filename: filename, kind: kind, sha256: "", size: 0,
        capturedAt: 0, contributor: "", favorite: false, caption: "", deleted: false, people: [],
        local: true, verifiedCopies: 1, availability: "Saved on this device")
    }
    let assets = [
      asset("red", "red.png", "photo"), asset("blue", "blue.png", "photo"),
      asset("audio", "tone.wav", "audio"),
    ]
    let project = FilmProject(
      roomId: "room", width: 128, height: 128,
      clips: [
        FilmClip(assetId: "red", duration: 1),
        FilmClip(assetId: "blue", track: 1, start: 0.4, duration: 0.4, opacity: 0.5),
        FilmClip(assetId: "audio", track: 2, start: 0.2, duration: 0.3, volume: 0.5),
      ])
    let output = root.appendingPathComponent("layers.mp4")
    try await FilmRenderer.render(
      project, assets: assets, urls: ["red": red, "blue": blue, "audio": audio], destination: output
    )
    let movie = AVURLAsset(url: output)
    let duration = try await movie.load(.duration)
    XCTAssertEqual(duration.seconds, 1, accuracy: 0.06)
    let generator = AVAssetImageGenerator(asset: movie)
    generator.requestedTimeToleranceBefore = .zero
    generator.requestedTimeToleranceAfter = .zero
    func channels(at seconds: Double) async throws -> [UInt8] {
      let frame = try await generator.image(at: CMTime(seconds: seconds, preferredTimescale: 600))
      let context = try XCTUnwrap(
        CGContext(
          data: nil, width: 1, height: 1, bitsPerComponent: 8,
          bytesPerRow: 4, space: CGColorSpaceCreateDeviceRGB(),
          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
      context.draw(frame.image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
      let values = try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self)
      return [values[0], values[1], values[2]]
    }
    let before = try await channels(at: 0.2)
    let during = try await channels(at: 0.6)
    let after = try await channels(at: 0.9)
    XCTAssertGreaterThan(before[0], 200)
    XCTAssertLessThan(before[2], 30)
    XCTAssertGreaterThan(during[0], 70)
    XCTAssertGreaterThan(during[2], 70)
    XCTAssertLessThan(
      during[1], min(during[0], during[2]) / 2,
      "The overlap must retain both source colors despite codec/color-space quantization.")
    XCTAssertGreaterThan(after[0], 200)
    XCTAssertLessThan(after[2], 30)
    let audioTracks = try await movie.loadTracks(withMediaType: .audio)
    XCTAssertEqual(audioTracks.count, 1)
    let reader = try AVAssetReader(asset: movie)
    let track = try XCTUnwrap(audioTracks.first)
    let trackOutput = AVAssetReaderTrackOutput(
      track: track,
      outputSettings: [
        AVFormatIDKey: kAudioFormatLinearPCM, AVLinearPCMIsFloatKey: true,
        AVLinearPCMBitDepthKey: 32, AVLinearPCMIsNonInterleaved: false,
      ])
    reader.add(trackOutput)
    XCTAssertTrue(reader.startReading())
    var maximum: Float = 0
    while let sample = trackOutput.copyNextSampleBuffer() {
      guard let block = CMSampleBufferGetDataBuffer(sample) else { continue }
      let length = CMBlockBufferGetDataLength(block)
      var values = [Float](repeating: 0, count: length / 4)
      let status = values.withUnsafeMutableBytes { bytes in
        CMBlockBufferCopyDataBytes(
          block, atOffset: 0, dataLength: length, destination: bytes.baseAddress!)
      }
      XCTAssertEqual(status, kCMBlockBufferNoErr)
      maximum = max(maximum, values.map { abs($0) }.max() ?? 0)
    }
    XCTAssertEqual(reader.status, .completed)
    XCTAssertGreaterThan(
      maximum, 0.05, "The movie must contain the mixed audio, not a silent track.")
    XCTAssertLessThan(maximum, 0.4, "The mix must apply the clip gain to the source tone.")
  }

}
