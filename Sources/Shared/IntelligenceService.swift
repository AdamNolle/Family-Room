import Foundation
import Vision
import CoreImage

actor FamilyIntelligenceService {
    static let shared = FamilyIntelligenceService()
    
    private init() {}
    
    /// Detects rectangles only. Recognition and embedding grouping are not implemented.
    func analyzeMedia(_ url: URL) async throws -> [DetectedFace] {
        let requestHandler = VNImageRequestHandler(url: url)
        let request = VNDetectFaceRectanglesRequest()
        
        try requestHandler.perform([request])
        
        guard let results = request.results else { return [] }
        
        return results.map { DetectedFace(bounds: $0.boundingBox, confidence: $0.confidence) }
    }
    
}

struct DetectedFace {
    let bounds: CGRect
    let confidence: Float
}
