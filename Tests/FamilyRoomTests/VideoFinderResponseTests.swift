import XCTest
@testable import FamilyRoom

final class VideoFinderResponseTests: XCTestCase {
    func testExactPhraseKeepsItsNearbyFrameClueSeparateWhenDecoded() throws {
        let payload = #"""
        {
          "answer": "I found spoken words at 00:10.0.",
          "matches": [{
            "room_id": "room",
            "asset_id": "birthday-video",
            "start_ms": 10000,
            "end_ms": 11500,
            "score": 232,
            "match_kind": "exact",
            "phrase_modality": "speech",
            "evidence": [{
              "modality": "speech",
              "start_ms": 10000,
              "end_ms": 11500,
              "text": "Happy birthday",
              "confidence_percent": 92
            }],
            "context_evidence": [{
              "modality": "visual_label",
              "start_ms": 10500,
              "end_ms": 12000,
              "text": "A child blowing out candles",
              "confidence_percent": 65
            }]
          }],
          "coverage": {
            "videos": 1,
            "indexed": 1,
            "audio_indexed": 1,
            "frames_indexed": 1,
            "sound_events_indexed": 0
          }
        }
        """#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase

        let response = try decoder.decode(VideoFinderResponse.self, from: Data(payload.utf8))
        let match = try XCTUnwrap(response.matches.first)

        XCTAssertEqual(match.matchKind, "exact")
        XCTAssertEqual(match.displayLabel, "Exact phrase · nearby clue")
        XCTAssertEqual(match.evidence.first?.modality, "speech")
        XCTAssertEqual(match.contextEvidence?.first?.modality, "visual_label")
    }
}
