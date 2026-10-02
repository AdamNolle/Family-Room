import XCTest

@MainActor
final class OnboardingTests: XCTestCase {
    func testCreateRoomAndNativeNavigation() throws {
        let app = XCUIApplication()
        app.launch()
        let create = app.buttons["onboarding.createRoom"]
        let settings = app.buttons["room.settings"]
        XCTAssertTrue(create.waitForExistence(timeout: 20) || settings.waitForExistence(timeout: 5))
        XCTAssertFalse(app.alerts.firstMatch.exists, "Opening the encrypted library must not produce a Keychain error.")
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        XCTAssertFalse(springboard.alerts.buttons["Allow Full Access"].waitForExistence(timeout: 2),
            "Opening Family Room must not request Photos access before a source is selected.")
        app.activate()
        XCTAssertEqual(app.state, .runningForeground)
        if create.exists {
            let enabled = XCTNSPredicateExpectation(predicate: NSPredicate(format: "isEnabled == true"), object: create)
            XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 20), .completed, "Keychain and catalog must finish opening.")
            create.tap()
        } else {
            XCTAssertFalse(app.buttons["Done"].exists, "Loading an existing Room must not reopen setup.")
            XCTAssertTrue(settings.isHittable)
            settings.tap()
        }
        let done = app.buttons["Done"]
        XCTAssertTrue(done.waitForExistence(timeout: 10))
        done.tap()
        XCTAssertTrue(app.tabBars.buttons["Home"].waitForExistence(timeout: 10))
        capture(app, name: "Empty Room Home")
        for label in ["Library", "Albums", "People", "Theater"] {
            let tab = app.tabBars.buttons[label]
            XCTAssertTrue(tab.exists, "Native primary navigation must include \(label).")
            tab.tap()
        }
        XCTAssertTrue(app.buttons["theater.createFilm"].exists)
        XCTAssertTrue(app.buttons["Add and manage memories"].exists)
        capture(app, name: "Theater")
    }

    private func capture(_ app: XCUIApplication, name: String) {
        guard ProcessInfo.processInfo.environment["FAMILY_ROOM_CAPTURE_UI"] == "1" else { return }
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
