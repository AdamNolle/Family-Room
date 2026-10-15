//! Owner-signed access changes and recipient-sealed epoch keys.
use crate::{
    crypto,
    error::{invalid, CoreError, Result},
    model::{AccessChange, Grant, Operation, OwnershipTransfer, Role, Room},
    signing,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use crypto_box::{PublicKey, SecretKey};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
struct Keys {
    room_id: String,
    revision: u64,
    epoch: u64,
    secrets: BTreeMap<u64, String>,
}
impl Drop for Keys {
    fn drop(&mut self) {
        for key in self.secrets.values_mut() {
            zeroize::Zeroize::zeroize(key);
        }
    }
}
fn public(device: &str) -> Result<PublicKey> {
    let bytes = crypto::parse_key(device)?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| CoreError::Authentication)?;
    let point = key.to_edwards();
    if key.is_weak() || !point.is_torsion_free() || point.compress().to_bytes() != bytes {
        return Err(CoreError::Authentication);
    }
    Ok(PublicKey::from_bytes(key.to_montgomery().to_bytes()))
}
fn secret(seed: &str) -> Result<SecretKey> {
    Ok(SecretKey::from_bytes(
        signing::identity(seed)?.to_scalar_bytes(),
    ))
}
pub(crate) fn seal(device: &str, bytes: &[u8]) -> Result<String> {
    let encrypted = public(device)?
        .seal(&mut rand::rngs::OsRng, bytes)
        .map_err(|_| CoreError::Authentication)?;
    Ok(URL_SAFE_NO_PAD.encode(encrypted))
}
pub(crate) fn unseal(seed: &str, encoded: &str) -> Result<Zeroizing<Vec<u8>>> {
    let encrypted = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| CoreError::Authentication)?;
    let bytes = secret(seed)?
        .unseal(&encrypted)
        .map_err(|_| CoreError::Authentication)?;
    Ok(Zeroizing::new(bytes))
}
pub(crate) fn hash<T: Serialize>(value: &T) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
fn message(change: &AccessChange) -> Result<Vec<u8>> {
    let mut unsigned = change.clone();
    unsigned.signature.clear();
    let mut bytes = b"family-room-access-v1\n".to_vec();
    bytes.extend(serde_json::to_vec(&unsigned)?);
    Ok(bytes)
}

fn transfer_message(transfer: &OwnershipTransfer) -> Result<Vec<u8>> {
    let mut unsigned = transfer.clone();
    unsigned.signature.clear();
    let mut bytes = b"family-room-owner-handoff-v1\n".to_vec();
    bytes.extend(serde_json::to_vec(&unsigned)?);
    Ok(bytes)
}

pub(crate) fn root_owner_id<'a>(chain: &'a [AccessChange], fallback: &'a str) -> &'a str {
    let Some(first) = chain.first() else {
        return fallback;
    };
    first
        .ownership_transfer
        .as_ref()
        .map_or(first.owner_id.as_str(), |transfer| {
            transfer.previous_owner_id.as_str()
        })
}

/// Find the authority that issued a grant retained in this access history.
pub(crate) fn grant_issuer<'a>(room: &'a Room, grant: &Grant) -> Option<&'a str> {
    for change in &room.access_changes {
        if change.members.get(&grant.device_id) == Some(grant) {
            return Some(&change.owner_id);
        }
        if let Some(transfer) = &change.ownership_transfer {
            if transfer.previous_members.get(&grant.device_id) == Some(grant) {
                return Some(&transfer.previous_owner_id);
            }
        }
    }
    room.access_changes
        .is_empty()
        .then_some(room.owner_id.as_str())
}

#[cfg(feature = "gateway")]
pub(crate) fn grant_was_issued_by(chain: &[AccessChange], grant: &Grant, issuer: &str) -> bool {
    let was_room_owner = chain.iter().any(|change| {
        change.owner_id == issuer
            || change
                .ownership_transfer
                .as_ref()
                .is_some_and(|transfer| transfer.previous_owner_id == issuer)
    });
    was_room_owner && signing::verify_grant(grant, issuer).is_ok()
}

fn validate_transfer_roster(transfer: &OwnershipTransfer, room: &str) -> Result<()> {
    if transfer.room_id != room
        || transfer.previous_owner_id == transfer.next_owner_id
        || transfer.previous_members.is_empty()
        || transfer.previous_members.len() > 256
        || !matches!(transfer.former_owner_role, Role::Admin | Role::Viewer)
        || !transfer
            .previous_members
            .contains_key(&transfer.previous_owner_id)
        || !transfer
            .previous_members
            .contains_key(&transfer.next_owner_id)
    {
        return Err(CoreError::Authentication);
    }
    for (device, grant) in &transfer.previous_members {
        if grant.device_id != *device
            || grant.room_id != room
            || (grant.role == Role::Owner) != (device == &transfer.previous_owner_id)
        {
            return Err(CoreError::Authentication);
        }
        public(device)?;
        signing::verify_grant(grant, &transfer.previous_owner_id)?;
    }
    signing::verify(
        &transfer.previous_owner_id,
        &transfer_message(transfer)?,
        &transfer.signature,
    )
}

pub(crate) fn verify_chain(
    chain: &[AccessChange],
    room: &str,
    root_owner: &str,
    expected_owner: &str,
) -> Result<()> {
    if chain.len() > 4096 {
        return Err(invalid("Access history exceeds the supported limit"));
    }
    let mut prior: Option<&AccessChange> = None;
    let mut current_owner = root_owner;
    for change in chain {
        let expected_revision = prior
            .map(|previous| previous.revision.checked_add(1))
            .unwrap_or(Some(1))
            .ok_or(CoreError::Authentication)?;
        let previous_hash = prior.map(hash).transpose()?.unwrap_or_default();
        let previous_epoch = prior.map_or(0, |previous| previous.epoch);
        if change.room_id != room
            || change.revision != expected_revision
            || change.previous != previous_hash
            || change.members.is_empty()
            || change.members.len() > 256
            || change.accepted_operations.len() > 1_000_000
            || change.envelopes.len() != change.members.len()
        {
            return Err(CoreError::Authentication);
        }

        if let Some(transfer) = &change.ownership_transfer {
            validate_transfer_roster(transfer, room)?;
            if transfer.previous_owner_id != current_owner
                || transfer.next_owner_id != change.owner_id
                || transfer.revision != change.revision
                || transfer.previous != change.previous
                || prior.is_some_and(|previous| transfer.previous_members != previous.members)
                || change.members.len() != transfer.previous_members.len()
                || change.epoch != previous_epoch.saturating_add(1)
            {
                return Err(CoreError::Authentication);
            }
            for (device, previous_grant) in &transfer.previous_members {
                let next = change
                    .members
                    .get(device)
                    .ok_or(CoreError::Authentication)?;
                let expected_role = if device == &transfer.next_owner_id {
                    Role::Owner
                } else if device == &transfer.previous_owner_id {
                    transfer.former_owner_role
                } else {
                    previous_grant.role
                };
                if next.role != expected_role {
                    return Err(CoreError::Authentication);
                }
            }
            current_owner = &change.owner_id;
        } else {
            if change.owner_id != current_owner
                || change.epoch < previous_epoch.max(1)
                || change.epoch > previous_epoch.saturating_add(1).max(1)
            {
                return Err(CoreError::Authentication);
            }
        }

        signing::verify(&change.owner_id, &message(change)?, &change.signature)?;
        for (device, grant) in &change.members {
            if grant.device_id != *device
                || grant.room_id != room
                || (grant.role == Role::Owner) != (device == &change.owner_id)
                || !change.envelopes.contains_key(device)
            {
                return Err(CoreError::Authentication);
            }
            public(device)?;
            signing::verify_grant(grant, &change.owner_id)?;
        }
        if !change.members.contains_key(&change.owner_id) {
            return Err(CoreError::Authentication);
        }
        for (id, digest) in &change.accepted_operations {
            if id.is_empty() || digest.len() != 64 || hex::decode(digest).is_err() {
                return Err(CoreError::Authentication);
            }
        }
        // History committed by an earlier access revision cannot be rewritten.
        if prior.is_some_and(|previous| {
            previous
                .accepted_operations
                .iter()
                .any(|(id, value)| change.accepted_operations.get(id) != Some(value))
        }) {
            return Err(CoreError::Authentication);
        }
        prior = Some(change);
    }
    if current_owner != expected_owner {
        return Err(CoreError::Authentication);
    }
    Ok(())
}

pub(crate) fn verify_extension(
    existing: &[AccessChange],
    incoming: &[AccessChange],
    room: &str,
    current_owner: &str,
) -> Result<bool> {
    let root_owner = root_owner_id(existing, current_owner);
    verify_chain(existing, room, root_owner, current_owner)?;
    // The empty chain is a valid prefix when an owner transfer was the first
    // access change. In that case it still belongs to the root owner, even
    // though the current owner has since changed.
    let next_owner = incoming
        .last()
        .map_or(root_owner, |change| change.owner_id.as_str());
    verify_chain(incoming, room, root_owner, next_owner)?;
    let common = existing.len().min(incoming.len());
    for index in 0..common {
        if hash(&existing[index])? != hash(&incoming[index])? {
            return Err(CoreError::Authentication);
        }
    }
    Ok(incoming.len() > existing.len())
}

pub(crate) fn epoch(room: &Room) -> u64 {
    room.access_changes.last().map_or(0, |p| p.epoch)
}
pub(crate) fn revision(room: &Room) -> u64 {
    room.access_changes.last().map_or(0, |p| p.revision)
}
pub(crate) fn key(room: &Room, epoch: u64) -> Result<[u8; 32]> {
    if epoch == 0 {
        return crypto::parse_key(&room.secret);
    }
    room.epoch_secrets
        .get(&epoch)
        .ok_or(CoreError::AccessDenied)
        .and_then(|s| crypto::parse_key(s))
}
pub(crate) fn allowed_device(room: &Room, device: &str) -> bool {
    room.access_changes
        .last()
        .is_none_or(|change| change.members.contains_key(device))
}
pub(crate) fn allowed_operation(room: &Room, op: &Operation) -> Result<bool> {
    let Some(change) = room.access_changes.last() else {
        return Ok(true);
    };
    if let Some(digest) = change.accepted_operations.get(&op.id) {
        return Ok(hash(op)? == *digest);
    }
    Ok(change
        .members
        .get(&op.author)
        .is_some_and(|g| g.role == op.grant.role && g.signature == op.grant.signature))
}
pub(crate) fn build(
    room: &Room,
    members: BTreeMap<String, Grant>,
    seed: &str,
    rotate: bool,
) -> Result<AccessChange> {
    if signing::device(seed)? != room.owner_id {
        return Err(CoreError::AccessDenied);
    }
    let revision = revision(room)
        .checked_add(1)
        .ok_or(CoreError::Authentication)?;
    let previous_epoch = epoch(room);
    let epoch = if previous_epoch == 0 || rotate {
        previous_epoch
            .checked_add(1)
            .ok_or(CoreError::Authentication)?
    } else {
        previous_epoch
    };
    let mut secrets = room.epoch_secrets.clone();
    secrets.insert(0, room.secret.clone());
    if epoch != previous_epoch {
        secrets.insert(epoch, hex::encode(crypto::key()));
    }
    let keys = Zeroizing::new(serde_json::to_vec(&Keys {
        room_id: room.id.clone(),
        revision,
        epoch,
        secrets,
    })?);
    let mut envelopes = BTreeMap::new();
    for device in members.keys() {
        envelopes.insert(device.clone(), seal(device, &keys)?);
    }
    let mut accepted_operations = room
        .access_changes
        .last()
        .map(|p| p.accepted_operations.clone())
        .unwrap_or_default();
    for op in &room.operations {
        accepted_operations.insert(op.id.clone(), hash(op)?);
    }
    let mut change = AccessChange {
        room_id: room.id.clone(),
        owner_id: room.owner_id.clone(),
        revision,
        epoch,
        previous: room
            .access_changes
            .last()
            .map(hash)
            .transpose()?
            .unwrap_or_default(),
        members,
        accepted_operations,
        envelopes,
        ownership_transfer: None,
        signature: String::new(),
    };
    change.signature = signing::sign_message(seed, &message(&change)?)?;
    let chain = [room.access_changes.clone(), vec![change.clone()]].concat();
    let root_owner = root_owner_id(&room.access_changes, &room.owner_id);
    verify_chain(&chain, &room.id, root_owner, &room.owner_id)?;
    Ok(change)
}

pub(crate) fn make_transfer_offer(
    room: &Room,
    members: BTreeMap<String, Grant>,
    next_owner_id: &str,
    seed: &str,
) -> Result<OwnershipTransfer> {
    let previous_owner_id = signing::device(seed)?;
    if previous_owner_id != room.owner_id
        || next_owner_id == previous_owner_id
        || !members.contains_key(next_owner_id)
        || members.len() > 256
    {
        return Err(CoreError::AccessDenied);
    }
    let mut transfer = OwnershipTransfer {
        room_id: room.id.clone(),
        previous_owner_id,
        next_owner_id: next_owner_id.into(),
        revision: revision(room)
            .checked_add(1)
            .ok_or(CoreError::Authentication)?,
        previous: room
            .access_changes
            .last()
            .map(hash)
            .transpose()?
            .unwrap_or_default(),
        previous_members: members,
        former_owner_role: if room.album_only {
            Role::Viewer
        } else {
            Role::Admin
        },
        signature: String::new(),
    };
    transfer.signature = signing::sign_message(seed, &transfer_message(&transfer)?)?;
    validate_transfer_roster(&transfer, &room.id)?;
    Ok(transfer)
}

pub(crate) fn build_transfer(
    room: &Room,
    transfer: OwnershipTransfer,
    seed: &str,
) -> Result<AccessChange> {
    let next_owner_id = signing::device(seed)?;
    let expected_previous = room
        .access_changes
        .last()
        .map(hash)
        .transpose()?
        .unwrap_or_default();
    if next_owner_id != transfer.next_owner_id
        || transfer.previous_owner_id != room.owner_id
        || transfer.room_id != room.id
        || transfer.revision != revision(room).saturating_add(1)
        || transfer.previous != expected_previous
        || (!room.access_changes.is_empty()
            && room.access_changes.last().map(|change| &change.members)
                != Some(&transfer.previous_members))
        || (room.access_changes.is_empty()
            && transfer.previous_members.get(&next_owner_id) != Some(&room.grant))
    {
        return Err(CoreError::Authentication);
    }
    validate_transfer_roster(&transfer, &room.id)?;

    let mut members = BTreeMap::new();
    for (device, previous_grant) in &transfer.previous_members {
        let role = if device == &transfer.next_owner_id {
            Role::Owner
        } else if device == &transfer.previous_owner_id {
            transfer.former_owner_role
        } else {
            previous_grant.role
        };
        members.insert(
            device.clone(),
            signing::grant(seed, &room.id, device, role)?,
        );
    }

    let revision = transfer.revision;
    let epoch = epoch(room)
        .checked_add(1)
        .ok_or(CoreError::Authentication)?;
    let mut secrets = room.epoch_secrets.clone();
    secrets.insert(0, room.secret.clone());
    secrets.insert(epoch, hex::encode(crypto::key()));
    let keys = Zeroizing::new(serde_json::to_vec(&Keys {
        room_id: room.id.clone(),
        revision,
        epoch,
        secrets,
    })?);
    let mut envelopes = BTreeMap::new();
    for device in members.keys() {
        envelopes.insert(device.clone(), seal(device, &keys)?);
    }
    let mut accepted_operations = room
        .access_changes
        .last()
        .map(|change| change.accepted_operations.clone())
        .unwrap_or_default();
    for operation in &room.operations {
        accepted_operations.insert(operation.id.clone(), hash(operation)?);
    }
    let mut change = AccessChange {
        room_id: room.id.clone(),
        owner_id: next_owner_id.clone(),
        revision,
        epoch,
        previous: transfer.previous.clone(),
        members,
        accepted_operations,
        envelopes,
        ownership_transfer: Some(transfer),
        signature: String::new(),
    };
    change.signature = signing::sign_message(seed, &message(&change)?)?;
    let chain = [room.access_changes.clone(), vec![change.clone()]].concat();
    let root_owner = root_owner_id(&room.access_changes, &room.owner_id);
    verify_chain(&chain, &room.id, root_owner, &next_owner_id)?;
    Ok(change)
}

pub(crate) fn apply(room: &mut Room, incoming: Vec<AccessChange>, seed: &str) -> Result<bool> {
    if !verify_extension(&room.access_changes, &incoming, &room.id, &room.owner_id)? {
        return Ok(false);
    }
    let last = incoming.last().ok_or(CoreError::Authentication)?;
    let device = signing::device(seed)?;
    if let Some(envelope) = last.envelopes.get(&device) {
        let keys: Keys = serde_json::from_slice(&unseal(seed, envelope)?)?;
        if keys.room_id != room.id
            || keys.revision != last.revision
            || keys.epoch != last.epoch
            || keys.secrets.get(&0) != Some(&room.secret)
            || !keys.secrets.contains_key(&last.epoch)
            || keys.secrets.keys().any(|epoch| *epoch > last.epoch)
        {
            return Err(CoreError::Authentication);
        }
        for value in keys.secrets.values() {
            crypto::parse_key(value)?;
        }
        for (epoch, value) in &room.epoch_secrets {
            if keys.secrets.get(epoch) != Some(value) {
                return Err(CoreError::Authentication);
            }
        }
        room.epoch_secrets = keys.secrets.clone();
        room.grant = last
            .members
            .get(&device)
            .ok_or(CoreError::Authentication)?
            .clone();
    }
    room.owner_id = last.owner_id.clone();
    room.access_changes = incoming;
    let mut retained = Vec::new();
    for op in &room.operations {
        if allowed_operation(room, op)? {
            retained.push(op.clone());
        } else {
            room.rejected_operations.push(op.clone());
        }
    }
    room.operations = retained;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_keys_are_recipient_bound_and_weak_device_keys_fail() {
        let seed = hex::encode(crypto::key());
        let device = signing::device(&seed).unwrap();
        let sealed = seal(&device, b"private epoch").unwrap();
        assert_eq!(&*unseal(&seed, &sealed).unwrap(), b"private epoch");
        assert!(unseal(&hex::encode(crypto::key()), &sealed).is_err());
        assert!(seal(&"00".repeat(32), b"secret").is_err());
    }
}
