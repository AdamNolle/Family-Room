//! Encrypted, local-first Family Room catalog shared by native clients.
mod access;
mod crypto;
mod engine;
mod error;
mod ffi;
#[cfg(feature = "gateway")]
pub mod gateway;
mod model;
mod providers;
mod s3;
mod signing;
mod timeline;
pub use error::CoreError;
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;
uniffi::setup_scaffolding!();

/// A serialized native command boundary. Clients must invoke it off the UI thread.
#[derive(uniffi::Object)]
pub struct FamilyCore {
    engine: Mutex<engine::Engine>,
}
#[uniffi::export]
impl FamilyCore {
    /// Opens an encrypted catalog. The native OS key store owns the recovery key.
    #[uniffi::constructor]
    pub fn open(directory: String, mut recovery_key: String) -> Result<Arc<Self>, CoreError> {
        let parsed = crypto::parse_key(&recovery_key);
        recovery_key.zeroize();
        let key = parsed?;
        Ok(Arc::new(Self {
            engine: Mutex::new(engine::Engine::open(directory, key)?),
        }))
    }
    /// Executes a version-one JSON command, returning a JSON value or a typed error.
    pub fn command(&self, request: String) -> Result<String, CoreError> {
        let input = serde_json::from_str(&request)?;
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| error::invalid("The catalog worker stopped; reopen the app"))?;
        Ok(serde_json::to_string(&engine.execute(input)?)?)
    }
}
/// Generates a cryptographically random recovery key; native clients store it securely.
#[uniffi::export]
pub fn new_recovery_key() -> String {
    hex::encode(crypto::key())
}

#[cfg(test)]
mod tests;
