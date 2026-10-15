#!/usr/bin/env swift
// Family Room build tool. Run from the repository root:
//
//   swift Scripts/build.swift test              Unit tests (SwiftPM)
//   swift Scripts/build.swift ios [simulator]   Build for the iOS Simulator; install and launch if a name is given
//   swift Scripts/build.swift mac [--run]       Build the Mac app; open it with --run
//   swift Scripts/build.swift verify            Tests plus both builds, as CI runs them
//   swift Scripts/build.swift icon              Re-render the app icon into the asset catalog
//
// Without a team in Config/Local.xcconfig, builds skip iCloud entitlements and keep the library on the device.

import Foundation

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let derived = ProcessInfo.processInfo.environment["FAMILY_ROOM_DERIVED"]
    ?? NSHomeDirectory() + "/Library/Caches/com.familyroom.build"
let project = "FamilyRoom.xcodeproj"

@discardableResult
func run(_ arguments: [String], quiet: Bool = false) -> Int32 {
    print("› " + arguments.joined(separator: " "))
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    process.arguments = arguments
    process.currentDirectoryURL = root
    if quiet { process.standardOutput = FileHandle.nullDevice }
    do { try process.run() } catch { print("Could not start \(arguments[0]): \(error)"); return 1 }
    process.waitUntilExit()
    return process.terminationStatus
}

func require(_ status: Int32, _ step: String) {
    guard status == 0 else { print("✗ \(step) failed"); exit(status) }
}

var team: String {
    guard let text = try? String(contentsOf: root.appendingPathComponent("Config/Local.xcconfig"), encoding: .utf8) else { return "" }
    for line in text.split(separator: "\n") where line.hasPrefix("DEVELOPMENT_TEAM") {
        return line.split(separator: "=").dropFirst().joined(separator: "=").trimmingCharacters(in: .whitespaces)
    }
    return ""
}

func test() {
    require(run(["swift", "test", "--build-system", "native", "--quiet"]), "Unit tests")
}

func ios(simulator: String?) {
    require(run(["xcodebuild", "-project", project, "-scheme", "FamilyRoom-iOS", "-configuration", "Debug",
                 "-destination", "generic/platform=iOS Simulator", "-derivedDataPath", derived,
                 "CODE_SIGNING_ALLOWED=NO", "-quiet", "build"]), "iOS build")
    let app = derived + "/Build/Products/Debug-iphonesimulator/Family Room.app"
    if let simulator {
        run(["xcrun", "simctl", "boot", simulator], quiet: true)
        require(run(["xcrun", "simctl", "install", simulator, app]), "Install")
        require(run(["xcrun", "simctl", "launch", simulator, "com.familyroom.app"]), "Launch")
    }
    print("✓ " + app)
}

func mac(open: Bool) {
    var arguments = ["xcodebuild", "-project", project, "-scheme", "FamilyRoom-macOS", "-configuration", "Debug",
                     "-derivedDataPath", derived, "-quiet"]
    if team.isEmpty {
        // No team: sign locally and leave out the iCloud entitlements, which need a provisioning profile.
        arguments += ["CODE_SIGN_IDENTITY=-", "CODE_SIGN_ENTITLEMENTS=", "CODE_SIGN_STYLE=Manual", "DEVELOPMENT_TEAM="]
    }
    require(run(arguments + ["build"]), "Mac build")
    let app = derived + "/Build/Products/Debug/Family Room.app"
    if open { run(["open", "-n", app]) }
    print("✓ " + app)
}

func icon() {
    let output = root.appendingPathComponent(".build/icon").path
    try? FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
    require(run(["swift", "Support/IconSource/render_icon.swift", output]), "Icon render")
    let set = "Support/Assets.xcassets/AppIcon.appiconset/"
    let copies = [("ios-1024.png", "AppIcon-iOS-1024.png")] + [16, 32, 64, 128, 256, 512, 1024].map { ("mac-\($0).png", "AppIcon-mac-\($0).png") }
    for (from, to) in copies {
        let destination = root.appendingPathComponent(set + to)
        try? FileManager.default.removeItem(at: destination)
        try! FileManager.default.copyItem(atPath: output + "/" + from, toPath: destination.path)
    }
    print("✓ Icon updated")
}

let arguments = Array(CommandLine.arguments.dropFirst())
switch arguments.first {
case "test": test()
case "ios": ios(simulator: arguments.dropFirst().first)
case "mac": mac(open: arguments.contains("--run"))
case "verify": test(); ios(simulator: nil); mac(open: false); print("✓ verify")
case "icon": icon()
default:
    print("usage: swift Scripts/build.swift test | ios [simulator] | mac [--run] | verify | icon")
    exit(2)
}
