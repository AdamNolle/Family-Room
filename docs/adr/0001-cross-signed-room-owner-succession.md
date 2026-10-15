# ADR 0001: Cross-signed Room owner succession

Status: Accepted

## Context

A Room owner may lose access to their device, need to hand administration to another family member, or want a different Member to manage the Room. Replacing the Room identity or asking every Member to rejoin would disrupt media history, storage sources, and offline devices. A succession operation must also prevent an unrelated Member from taking ownership and prevent two stale offers from silently creating competing owners.

## Decision

The current owner prepares a signed offer that names exactly one existing Member device and binds the offer to the current access revision and chain head. Only the named device may accept it. Acceptance creates a new access revision signed by the successor. The revision retains the existing roster, reissues its grants, rotates the key epoch, promotes the successor to Owner, and demotes the outgoing owner to Admin (or Viewer for an album-only Room).

The original owner remains the trust anchor for the append-only access chain. Existing gateways accept only a valid extension of their stored chain. A newly connected gateway is initialized from the complete signed history, preserving the original anchor through succession. Valid but competing offers may both be accepted offline; their branches cannot merge or replace a gateway policy that has already advanced. The UI reports local acceptance separately from successful publication to connected storage.

## Consequences

- Room identity, media history, Member set, and existing storage configuration survive succession.
- Future contributions use a fresh epoch key; existing Members receive the sealed history they need to read prior media.
- Previously downloaded media remains readable by its recipient.
- A transfer requires an existing Member device to receive and accept the signed offer. Removing or adding Members is a separate operation.
- Offline devices and gateways learn the new owner when a valid signed update reaches them; universal instantaneous revocation is not promised.
- Competing offline acceptance is preserved as a conflict instead of silently choosing a winner.
- Local device departure forgets that device’s active Room keys and connections but does not rewrite the owner-signed roster. An owner must revoke the device for Room-wide removal. Independent cryptographic review remains separate work.

## Validation

The core suite verifies target-device binding, fork rejection, grant reissue, former-owner demotion, key-epoch rotation, recovery after reopening, gateway publication, existing-member refresh, successor invitations, and registration of a fresh gateway from the signed ownership history.
