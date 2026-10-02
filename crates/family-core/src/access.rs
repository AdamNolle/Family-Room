//! Owner-signed access changes and recipient-sealed epoch keys.
use crate::{
    crypto,
    error::{invalid, CoreError, Result},
    model::{AccessChange, Grant, Operation, Role, Room},
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
pub(crate) fn verify_chain(chain: &[AccessChange], room: &str, owner: &str) -> Result<()> {
    if chain.len() > 4096 {
        return Err(invalid("Access history exceeds the supported limit"));
    }
    let mut prior: Option<&AccessChange> = None;
    for change in chain {
        if change.room_id != room
            || change.owner_id != owner
            || change.revision != prior.map_or(1, |p| p.revision + 1)
            || change.epoch < prior.map_or(1, |p| p.epoch)
            || change.epoch > prior.map_or(1, |p| p.epoch.saturating_add(1))
            || change.previous != prior.map(hash).transpose()?.unwrap_or_default()
            || change.members.is_empty()
            || change.members.len() > 256
            || change.accepted_operations.len() > 1000000
            || change.envelopes.len() != change.members.len()
        {
            return Err(CoreError::Authentication);
        }
        signing::verify(owner, &message(change)?, &change.signature)?;
        for (device, grant) in &change.members {
            if grant.device_id != *device
                || grant.room_id != room
                || (grant.role == Role::Owner) != (device == owner)
                || !change.envelopes.contains_key(device)
            {
                return Err(CoreError::Authentication);
            }
            public(device)?;
            signing::verify_grant(grant, owner)?;
        }
        if !change.members.contains_key(owner) {
            return Err(CoreError::Authentication);
        }
        for (id, digest) in &change.accepted_operations {
            if id.is_empty() || digest.len() != 64 || hex::decode(digest).is_err() {
                return Err(CoreError::Authentication);
            }
        }
        // History committed by an earlier access revision cannot be rewritten.
        if prior.is_some_and(|p| {
            p.accepted_operations
                .iter()
                .any(|(id, value)| change.accepted_operations.get(id) != Some(value))
        }) {
            return Err(CoreError::Authentication);
        }
        prior = Some(change);
    }
    Ok(())
}
pub(crate) fn verify_extension(
    existing: &[AccessChange],
    incoming: &[AccessChange],
    room: &str,
    owner: &str,
) -> Result<bool> {
    verify_chain(incoming, room, owner)?;
    let common = existing.len().min(incoming.len());
    for i in 0..common {
        if hash(&existing[i])? != hash(&incoming[i])? {
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
        signature: String::new(),
    };
    change.signature = signing::sign_message(seed, &message(&change)?)?;
    verify_chain(
        &[room.access_changes.clone(), vec![change.clone()]].concat(),
        &room.id,
        &room.owner_id,
    )?;
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
