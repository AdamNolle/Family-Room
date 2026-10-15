//! Pointer-free C ABI used by Windows. Each transport word contains up to eight UTF-8 bytes.
//! Handles are opaque, bounded, and explicitly released; no foreign pointers are dereferenced.
use crate::FamilyCore;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex, OnceLock},
};

const MAX_REQUEST: usize = 8 * 1024 * 1024;
const MAX_HANDLES: usize = 256;
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
struct Registry {
    requests: BTreeMap<u64, Vec<u8>>,
    responses: BTreeMap<u64, Vec<u8>>,
    cores: BTreeMap<u64, Arc<FamilyCore>>,
}
fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}
fn response(value: serde_json::Value) -> u64 {
    let Ok(mut registry) = registry().lock() else {
        return 0;
    };
    if registry.responses.len() >= MAX_HANDLES {
        return 0;
    }
    let Ok(bytes) = serde_json::to_vec(&value) else {
        return 0;
    };
    let handle = NEXT.fetch_add(1, Ordering::Relaxed);
    registry.responses.insert(handle, bytes);
    handle
}
/// Allocates an empty request buffer. Returns zero if the transport is exhausted.
#[no_mangle]
pub extern "C" fn fr_request_new() -> u64 {
    let Ok(mut r) = registry().lock() else {
        return 0;
    };
    if r.requests.len() >= MAX_HANDLES {
        return 0;
    }
    let h = NEXT.fetch_add(1, Ordering::Relaxed);
    r.requests.insert(h, vec![]);
    h
}
/// Appends the low `length` bytes of a little-endian transport word.
#[no_mangle]
pub extern "C" fn fr_request_push(handle: u64, word: u64, length: u8) -> u8 {
    if length > 8 {
        return 0;
    }
    let Ok(mut r) = registry().lock() else {
        return 0;
    };
    let Some(bytes) = r.requests.get_mut(&handle) else {
        return 0;
    };
    if bytes.len() + usize::from(length) > MAX_REQUEST {
        return 0;
    }
    bytes.extend_from_slice(&word.to_le_bytes()[..usize::from(length)]);
    1
}
/// Consumes a request. With core zero, accepts directory/recovery_key and opens a vault.
#[no_mangle]
pub extern "C" fn fr_dispatch(core: u64, request: u64) -> u64 {
    let (bytes, instance) = {
        let Ok(mut r) = registry().lock() else {
            return 0;
        };
        (r.requests.remove(&request), r.cores.get(&core).cloned())
    };
    let Some(bytes) = bytes else {
        return response(json!({"ok":false,"error":"Invalid request handle"}));
    };
    let result = (|| -> std::result::Result<serde_json::Value, String> {
        let input = String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 request")?;
        if core == 0 {
            let value: serde_json::Value =
                serde_json::from_str(&input).map_err(|_| "Invalid open request")?;
            let directory = value["directory"].as_str().ok_or("Missing directory")?;
            let key = value["recovery_key"]
                .as_str()
                .ok_or("Missing recovery key")?;
            let instance =
                FamilyCore::open(directory.into(), key.into()).map_err(|e| e.to_string())?;
            let mut r = registry().lock().map_err(|_| "The transport stopped")?;
            if r.cores.len() >= MAX_HANDLES {
                return Err("Too many open libraries".into());
            }
            let h = NEXT.fetch_add(1, Ordering::Relaxed);
            r.cores.insert(h, instance);
            Ok(json!({"handle":h}))
        } else {
            let instance = instance.ok_or("Library is closed")?;
            let output = instance.command(input).map_err(|e| e.to_string())?;
            serde_json::from_str(&output).map_err(|_| "Invalid core response".into())
        }
    })();
    match result {
        Ok(value) => response(json!({"ok":true,"result":value})),
        Err(error) => response(json!({"ok":false,"error":error})),
    }
}
/// Returns the response length. Invalid handles have length zero.
#[no_mangle]
pub extern "C" fn fr_response_length(handle: u64) -> u64 {
    registry()
        .lock()
        .ok()
        .and_then(|r| r.responses.get(&handle).map(|b| b.len() as u64))
        .unwrap_or(0)
}
/// Returns eight bytes starting at `word_index * 8`, padded with zeros.
#[no_mangle]
pub extern "C" fn fr_response_word(handle: u64, word_index: u64) -> u64 {
    let Ok(r) = registry().lock() else { return 0 };
    let Some(bytes) = r.responses.get(&handle) else {
        return 0;
    };
    let Some(offset) = word_index
        .checked_mul(8)
        .and_then(|v| usize::try_from(v).ok())
    else {
        return 0;
    };
    if offset >= bytes.len() {
        return 0;
    }
    let mut word = [0; 8];
    let n = (bytes.len() - offset).min(8);
    word[..n].copy_from_slice(&bytes[offset..offset + n]);
    u64::from_le_bytes(word)
}
/// Releases a request, response, or vault handle. Unknown handles are harmless.
#[no_mangle]
pub extern "C" fn fr_release(handle: u64) {
    if let Ok(mut r) = registry().lock() {
        r.requests.remove(&handle);
        r.responses.remove(&handle);
        r.cores.remove(&handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_abi_round_trips_without_foreign_pointers() {
        let request = fr_request_new();
        assert_eq!(
            fr_request_push(request, u64::from_le_bytes(*b"abcdefgh"), 9),
            0
        );
        let text = b"{\"action\":\"snapshot\"}";
        for chunk in text.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            assert_eq!(
                fr_request_push(request, u64::from_le_bytes(word), chunk.len() as u8),
                1
            );
        }
        let response = fr_dispatch(123, request);
        let length = fr_response_length(response) as usize;
        let mut bytes = vec![];
        for i in 0..length.div_ceil(8) {
            bytes.extend(fr_response_word(response, i as u64).to_le_bytes());
        }
        bytes.truncate(length);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["ok"], false);
        fr_release(response);
        assert_eq!(fr_response_length(response), 0);
    }
}
