import CloudKit
import CoreData
import CoreTransferable
import SwiftUI

/// A Room member as seen through its CloudKit share.
struct RoomMember: Identifiable, Hashable {
    enum Status: String { case syncing = "Syncing", pending = "Invitation pending", removed = "Removed", you = "You" }
    var id: String
    var name: String
    var isOwner: Bool
    var isCurrentUser: Bool
    var status: Status
    var initial: String { String(name.prefix(1)).uppercased() }
}

/// Wraps a CKShare so SwiftUI's ShareLink can hand out Room invitations through the system share sheet.
struct RoomInvitation: Transferable {
    let share: CKShare
    let container: CKContainer

    static var transferRepresentation: some TransferRepresentation {
        CKShareTransferRepresentation { invitation in
            .existing(invitation.share, container: invitation.container)
        }
    }
}

@MainActor
final class RoomSharing: ObservableObject {
    @Published private(set) var share: CKShare?
    @Published private(set) var members: [RoomMember] = []
    @Published private(set) var accountStatus: CKAccountStatus = .couldNotDetermine
    @Published private(set) var error: String?
    @Published private(set) var busy = false

    let persistence: Persistence
    var ckContainer: CKContainer { CKContainer(identifier: Persistence.containerIdentifier) }

    init(persistence: Persistence) {
        self.persistence = persistence
    }

    var available: Bool { persistence.cloudEnabled }

    func refresh(for room: Room?) async {
        guard available, let room else { share = nil; members = []; return }
        do {
            accountStatus = try await ckContainer.accountStatus()
        } catch { accountStatus = .couldNotDetermine }
        do {
            let shares = try persistence.container.fetchShares(matching: [room.objectID])
            share = shares[room.objectID]
            members = Self.members(of: share)
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Creates the share the first time a Room is offered to someone else.
    func invitation(for room: Room) async -> RoomInvitation? {
        guard available else { return nil }
        busy = true; defer { busy = false }
        do {
            if share == nil {
                let (_, newShare, _) = try await persistence.container.share([room], to: nil)
                newShare[CKShare.SystemFieldKey.title] = room.displayName
                if let store = persistence.privateStore {
                    try await persistence.container.persistUpdatedShare(newShare, in: store)
                }
                share = newShare
                members = Self.members(of: newShare)
            }
            guard let share else { return nil }
            return RoomInvitation(share: share, container: ckContainer)
        } catch {
            self.error = error.localizedDescription
            return nil
        }
    }

    func removeParticipant(_ member: RoomMember, from room: Room) async {
        guard let share, let participant = share.participants.first(where: { Self.identifier(for: $0) == member.id }), let store = persistence.privateStore else { return }
        share.removeParticipant(participant)
        do {
            try await persistence.container.persistUpdatedShare(share, in: store)
            await refresh(for: room)
        } catch { self.error = error.localizedDescription }
    }

    func leave(_ room: Room) async {
        guard let store = persistence.sharedStore else { return }
        do {
            guard let zone = persistence.container.recordID(for: room.objectID)?.zoneID else { return }
            try await persistence.container.purgeObjectsAndRecordsInZone(with: zone, in: store)
        } catch { self.error = error.localizedDescription }
    }

    static func identifier(for participant: CKShare.Participant) -> String {
        participant.userIdentity.userRecordID?.recordName
            ?? participant.userIdentity.lookupInfo?.emailAddress
            ?? participant.userIdentity.lookupInfo?.phoneNumber
            ?? UUID().uuidString
    }

    static func members(of share: CKShare?) -> [RoomMember] {
        guard let share else { return [] }
        return share.participants.map { p in
            let identity = p.userIdentity
            let formatted = identity.nameComponents.map { PersonNameComponentsFormatter.localizedString(from: $0, style: .default) } ?? ""
            let name = formatted.isEmpty ? (identity.lookupInfo?.emailAddress ?? identity.lookupInfo?.phoneNumber ?? "Family member") : formatted
            let status: RoomMember.Status
            if p == share.currentUserParticipant { status = .you }
            else {
                switch p.acceptanceStatus {
                case .accepted: status = .syncing
                case .pending, .unknown: status = .pending
                case .removed: status = .removed
                @unknown default: status = .pending
                }
            }
            return RoomMember(id: identifier(for: p), name: name, isOwner: p.role == .owner,
                              isCurrentUser: p == share.currentUserParticipant, status: status)
        }
        .sorted { a, b in a.isOwner != b.isOwner ? a.isOwner : a.name < b.name }
    }

    var accountMessage: String {
        guard available else { return "iCloud is off in this build, so this library stays on this \(DeviceWord.current)." }
        switch accountStatus {
        case .available: return "Signed in to iCloud. Changes sync to every device in the Room."
        case .noAccount: return "Sign in to iCloud in System Settings to sync and invite family."
        case .restricted: return "iCloud is restricted on this device."
        case .temporarilyUnavailable: return "iCloud is temporarily unavailable. Changes are kept and sent later."
        default: return "Checking iCloud…"
        }
    }
}

