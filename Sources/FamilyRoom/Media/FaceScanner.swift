import CoreData
import CoreGraphics
import Foundation
import ImageIO
import Vision

/// One face found in a photo, with a feature print used only to group similar faces on this device.
struct DetectedFace {
    var box: CGRect           // normalized, origin bottom-left as Vision reports
    var confidence: Double
    var crop: Data?
    var featurePrint: Data?
}

enum FaceScanner {
    /// Faces whose feature-print distance is under this are considered the same person.
    static let groupingDistance: Float = 0.62

    static func detect(in imageData: Data) async throws -> [DetectedFace] {
        guard let image = MediaImporter.decode(imageData, maxSide: 1600) else { return [] }
        let request = VNDetectFaceRectanglesRequest()
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        try handler.perform([request])
        var faces: [DetectedFace] = []
        for observation in request.results ?? [] where observation.confidence > 0.5 {
            let box = observation.boundingBox
            let crop = cropFace(image, box: box)
            var print: Data?
            if let crop, let cropImage = MediaImporter.decode(crop, maxSide: 320) {
                let fp = VNGenerateImageFeaturePrintRequest()
                try? VNImageRequestHandler(cgImage: cropImage, options: [:]).perform([fp])
                if let result = fp.results?.first as? VNFeaturePrintObservation {
                    print = try? NSKeyedArchiver.archivedData(withRootObject: result, requiringSecureCoding: true)
                }
            }
            faces.append(DetectedFace(box: box, confidence: Double(observation.confidence), crop: crop, featurePrint: print))
        }
        return faces
    }

    private static func cropFace(_ image: CGImage, box: CGRect) -> Data? {
        let w = CGFloat(image.width), h = CGFloat(image.height)
        let pad = 0.35
        var rect = CGRect(x: box.minX * w, y: (1 - box.maxY) * h, width: box.width * w, height: box.height * h)
        rect = rect.insetBy(dx: -rect.width * pad, dy: -rect.height * pad).intersection(CGRect(x: 0, y: 0, width: w, height: h))
        guard let cropped = image.cropping(to: rect.integral) else { return nil }
        return MediaImporter.jpeg(cropped, quality: 0.85)
    }

    static func observation(_ data: Data) -> VNFeaturePrintObservation? {
        try? NSKeyedUnarchiver.unarchivedObject(ofClass: VNFeaturePrintObservation.self, from: data)
    }

    static func closest(to print: VNFeaturePrintObservation, in known: [(person: Person, print: VNFeaturePrintObservation)]) -> Person? {
        var best: (Person, Float)?
        for candidate in known {
            var d: Float = 0
            guard (try? print.computeDistance(&d, to: candidate.print)) != nil else { continue }
            if d < groupingDistance, best == nil || d < best!.1 { best = (candidate.person, d) }
        }
        return best?.0
    }

    static func distance(_ a: Data, _ b: Data) -> Float? {
        guard let pa = try? NSKeyedUnarchiver.unarchivedObject(ofClass: VNFeaturePrintObservation.self, from: a),
              let pb = try? NSKeyedUnarchiver.unarchivedObject(ofClass: VNFeaturePrintObservation.self, from: b) else { return nil }
        var d: Float = 0
        try? pa.computeDistance(&d, to: pb)
        return d
    }
}

extension RoomStore {
    /// Scans photos in the Room that have not been checked yet and groups faces into face groups.
    /// Detection runs off the main thread; feature prints stay in the Room's own library and never go to a service.
    func scanFaces(in room: Room) async {
        guard faceScanProgress == nil else { return }
        let pending = room.mediaList.filter { !$0.isVideo && !$0.faceScanned && $0.data != nil }
        let progress = Progress(totalUnitCount: Int64(pending.count))
        faceScanProgress = progress
        defer { faceScanProgress = nil }
        // Decode every known print once instead of once per comparison.
        var known: [(person: Person, print: VNFeaturePrintObservation)] = []
        for person in room.people ?? [] {
            for face in person.faces ?? [] {
                if let data = face.featurePrint, let print = FaceScanner.observation(data) { known.append((person, print)) }
            }
        }
        for item in pending {
            if Task.isCancelled { break }
            guard let data = item.data else { continue }
            let found = (try? await Task.detached(priority: .utility) { try await FaceScanner.detect(in: data) }.value) ?? []
            for face in found {
                let obs = insert(FaceObservation.self, near: item)
                obs.id = UUID(); obs.confidence = face.confidence
                obs.boxX = face.box.minX; obs.boxY = face.box.minY; obs.boxWidth = face.box.width; obs.boxHeight = face.box.height
                obs.crop = face.crop; obs.featurePrint = face.featurePrint; obs.media = item
                let print = face.featurePrint.flatMap(FaceScanner.observation)
                if let print { obs.person = FaceScanner.closest(to: print, in: known) }
                if obs.person == nil, print != nil {
                    let groups = (room.people ?? []).filter(\.isFaceGroup).count
                    obs.person = createPerson(named: "Face \(groups + 1)", faceGroup: true, in: room)
                }
                if let person = obs.person, let print { known.append((person, print)) }
                if let person = obs.person, !person.isFaceGroup, let moment = item.moment {
                    var set = moment.people ?? []; set.insert(person); moment.people = set
                }
            }
            item.faceScanned = true
            progress.completedUnitCount += 1
            save()
        }
    }
}
