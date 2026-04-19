// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "FamilyRoom",
    platforms: [
        .macOS(.v14), .iOS(.v17)
    ],
    products: [
        .executable(name: "FamilyRoom", targets: ["FamilyRoom"])
    ],
    targets: [
        .executableTarget(
            name: "FamilyRoom",
            path: "Sources/Shared"
        )
    ]
)
