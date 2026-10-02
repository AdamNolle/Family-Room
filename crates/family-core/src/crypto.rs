use crate::error::{invalid, CoreError, Result};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub const CHUNK: usize = 1024 * 1024;
pub fn key() -> [u8; 32] {
    let mut k = [0; 32];
    OsRng.fill_bytes(&mut k);
    k
}
pub fn parse_key(s: &str) -> Result<[u8; 32]> {
    hex::decode(s.trim().replace([' ', '-'], ""))
        .map_err(|_| invalid("Use a 64-character recovery key"))?
        .try_into()
        .map_err(|_| invalid("Use a 64-character recovery key"))
}
pub fn derive(secret: &[u8; 32], context: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(secret);
    hash.update(context);
    hash.finalize().into()
}
pub fn encrypt(secret: &[u8; 32], plain: &[u8], context: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0; 24];
    OsRng.fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new(secret.into());
    let data = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plain,
                aad: context,
            },
        )
        .map_err(|_| CoreError::Authentication)?;
    let mut result = b"FRE1".to_vec();
    result.extend(nonce);
    result.extend(data);
    Ok(result)
}
pub fn decrypt(secret: &[u8; 32], data: &[u8], context: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 44 || &data[..4] != b"FRE1" {
        return Err(CoreError::Authentication);
    }
    XChaCha20Poly1305::new(secret.into())
        .decrypt(
            XNonce::from_slice(&data[4..28]),
            Payload {
                msg: &data[28..],
                aad: context,
            },
        )
        .map_err(|_| CoreError::Authentication)
}
pub fn digest(path: &Path) -> Result<(String, u64)> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0; CHUNK];
    let mut total = 0;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        total += n as u64;
    }
    Ok((hex::encode(h.finalize()), total))
}
pub fn object_id(secret: &[u8; 32], digest: &str) -> Result<String> {
    let mut m =
        <Hmac<Sha256> as Mac>::new_from_slice(secret).map_err(|_| CoreError::Authentication)?;
    m.update(digest.as_bytes());
    Ok(hex::encode(m.finalize().into_bytes()))
}
pub fn encrypt_file(secret: &[u8; 32], source: &Path, target: &Path, context: &str) -> Result<()> {
    let mut input = File::open(source)?;
    let mut output = File::create(target)?;
    output.write_all(b"FRB1")?;
    output.write_all(&input.metadata()?.len().to_le_bytes())?;
    let mut buf = vec![0; CHUNK];
    let mut index = 0u64;
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let encrypted = encrypt(secret, &buf[..n], format!("{context}:{index}").as_bytes())?;
        output.write_all(&(encrypted.len() as u32).to_le_bytes())?;
        output.write_all(&encrypted)?;
        index += 1;
    }
    output.sync_all()?;
    Ok(())
}
pub fn decrypt_file(
    secret: &[u8; 32],
    source: &Path,
    target: &Path,
    context: &str,
    expected_hash: &str,
) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| invalid("Missing destination directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut output = tempfile::NamedTempFile::new_in(parent)?;
    decrypt_to(secret, source, &mut output, context, expected_hash)?;
    output.as_file().sync_all()?;
    output
        .persist(target)
        .map_err(|e| CoreError::from(e.error))?;
    Ok(())
}

/// Authenticate every chunk and the original's hash without retaining plaintext.
pub fn verify_file(
    secret: &[u8; 32],
    source: &Path,
    context: &str,
    expected_hash: &str,
) -> Result<u64> {
    decrypt_to(secret, source, &mut std::io::sink(), context, expected_hash)
}

fn decrypt_to(
    secret: &[u8; 32],
    source: &Path,
    output: &mut impl Write,
    context: &str,
    expected_hash: &str,
) -> Result<u64> {
    let mut input = File::open(source)?;
    let mut header = [0; 12];
    input
        .read_exact(&mut header)
        .map_err(ciphertext_read_error)?;
    if &header[..4] != b"FRB1" {
        return Err(CoreError::Authentication);
    }
    let length = u64::from_le_bytes(
        header[4..12]
            .try_into()
            .map_err(|_| CoreError::Authentication)?,
    );
    let mut index = 0;
    let mut written = 0;
    let mut hash = Sha256::new();
    while written < length {
        let mut size = [0; 4];
        input.read_exact(&mut size).map_err(ciphertext_read_error)?;
        let n = u32::from_le_bytes(size) as usize;
        if !(44..=CHUNK + 44).contains(&n) {
            return Err(CoreError::Authentication);
        }
        let mut buf = vec![0; n];
        input.read_exact(&mut buf).map_err(ciphertext_read_error)?;
        let plain = decrypt(secret, &buf, format!("{context}:{index}").as_bytes())?;
        if plain.is_empty() || written + plain.len() as u64 > length {
            return Err(CoreError::Authentication);
        }
        written += plain.len() as u64;
        hash.update(&plain);
        output.write_all(&plain)?;
        index += 1;
    }
    let mut extra = [0; 1];
    if input.read(&mut extra)? != 0 || hex::encode(hash.finalize()) != expected_hash {
        return Err(CoreError::Authentication);
    }
    Ok(written)
}

fn ciphertext_read_error(error: std::io::Error) -> CoreError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        CoreError::Authentication
    } else {
        error.into()
    }
}
