# Technical Approach: The "Cozy Glass" Framework

## 1. Design Aesthetic: "Cozy Glass"
The core challenge is merging the sleek, premium feel of "Liquid Glass" with a "Cozy Fabric" atmosphere.

### UI Techniques
- **Frosted Silk**: Using `ultraThinMaterial` with a custom tint of `Color(white: 0.95)` and high saturation to mimic fine linen.
- **Organic Shadows**: Replacing standard drop shadows with large, diffused, colored shadows that mimic natural lighting.
- **Ambient Lighting**: Using `LinearGradients` and `MeshGradients` that transition between soft ambers and muted creams.
- **Haptics & Motion**: Using `.spring(response: 0.6, dampingFraction: 0.8)` for all transitions to give the UI a sense of physical weight and softness.

## 2. Cross-Platform Core
The app will use a unified SwiftUI codebase to target iPhone, iPad, and Mac.

### Platform-Specific UI
- **iPhone**: Tab-based navigation with a floating "Liquid Glass" bar.
- **iPad/Mac**: Adaptive Sidebar with a hierarchical view of Family Rooms.

## 3. Local Intelligence (Privacy First)
Following the "Family Room" philosophy, no data is processed on a server.
- **Face Detection**: Uses Vision's `VNDetectFaceRectanglesRequest` and `VNGenerateFacePlaceholdersRequest` for grouping.
- **Smart Stitching**: An actor-based service that uses `AVMutableComposition` to intelligently find the "best" clips based on smiling faces or dynamic motion.

## 4. Collaborative Sync (CloudKit)
- **Shared Data**: Uses CloudKit's `CKShare` to manage invitations.
- **Real-time Updates**: SwiftData's CloudKit integration handles conflict resolution and live syncing between family devices without custom backend code.
