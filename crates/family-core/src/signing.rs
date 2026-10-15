use crate::{
    crypto,
    error::{CoreError, Result},
    model::{Grant, Operation, Role},
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub fn identity(seed: &str) -> Result<SigningKey> {
    Ok(SigningKey::from_bytes(&crypto::parse_key(seed)?))
}
pub fn device(seed: &str) -> Result<String> {
    Ok(hex::encode(identity(seed)?.verifying_key().as_bytes()))
}
pub(crate) fn verify(public: &str, data: &[u8], sig: &str) -> Result<()> {
    let key_bytes = crypto::parse_key(public)?;
    let public = VerifyingKey::from_bytes(&key_bytes).map_err(|_| CoreError::Authentication)?;
    let bytes = hex::decode(sig).map_err(|_| CoreError::Authentication)?;
    let signature = Signature::from_slice(&bytes).map_err(|_| CoreError::Authentication)?;
    public
        .verify_strict(data, &signature)
        .map_err(|_| CoreError::Authentication)
}
pub(crate) fn sign_message(seed: &str, data: &[u8]) -> Result<String> {
    Ok(hex::encode(identity(seed)?.sign(data).to_bytes()))
}
pub fn grant(seed: &str, room_id: &str, device_id: &str, role: Role) -> Result<Grant> {
    crypto::parse_key(device_id)?;
    let mut g = Grant {
        room_id: room_id.into(),
        device_id: device_id.into(),
        role,
        signature: String::new(),
    };
    g.signature = hex::encode(identity(seed)?.sign(&serde_json::to_vec(&g)?).to_bytes());
    Ok(g)
}
pub fn verify_grant(g: &Grant, owner: &str) -> Result<()> {
    let mut unsigned = g.clone();
    unsigned.signature.clear();
    verify(owner, &serde_json::to_vec(&unsigned)?, &g.signature)
}
pub fn sign_operation(op: &mut Operation, seed: &str) -> Result<()> {
    op.signature.clear();
    op.signature = hex::encode(identity(seed)?.sign(&serde_json::to_vec(op)?).to_bytes());
    Ok(())
}
pub fn verify_operation(op: &Operation, owner: &str, room_id: &str) -> Result<()> {
    if op.room_id != room_id || op.grant.room_id != room_id || op.grant.device_id != op.author {
        return Err(CoreError::AccessDenied);
    }
    verify_grant(&op.grant, owner)?;
    if op.grant.role == Role::Owner && op.author != owner {
        return Err(CoreError::AccessDenied);
    }
    if op.grant.role == Role::Viewer && !["comment", "reaction"].contains(&op.kind.as_str()) {
        return Err(CoreError::AccessDenied);
    }
    let mut unsigned = op.clone();
    unsigned.signature.clear();
    verify(&op.author, &serde_json::to_vec(&unsigned)?, &op.signature)
}
