// swift-tools-version: 5.9
import PackageDescription
import Foundation

let packageRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "FamilyRoom",
    platforms: [
        .macOS(.v14), .iOS(.v17)
    ],
    products: [
        .executable(name: "FamilyRoom", targets: ["FamilyRoom"])
    ],
    targets: [
        .systemLibrary(name: "family_coreFFI", path: "Generated/Swift"),
        .target(
            name: "FamilyCoreBindings",
            dependencies: ["family_coreFFI"],
            path: "Generated/Swift",
            exclude: ["family_coreFFI.h", "family_coreFFI.modulemap", "module.modulemap"],
            sources: ["family_core.swift"],
            linkerSettings: [
                .linkedLibrary("family_core"),
                .unsafeFlags(["-L\(packageRoot)/target/debug", "-Xlinker", "-rpath", "-Xlinker", "\(packageRoot)/target/debug"], .when(platforms: [.macOS]))
            ]
        ),
        .executableTarget(
            name: "FamilyRoom",
            dependencies: ["FamilyCoreBindings"],
            path: "Sources/Shared"
        ),
        .testTarget(name: "FamilyRoomTests", dependencies: ["FamilyRoom"], path: "Tests/FamilyRoomTests")
    ]
)
