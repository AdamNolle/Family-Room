use thiserror::Error;

/// Errors crossing the native boundary never include credentials or media contents.
#[derive(Debug, Error, uniffi::Error)]
pub enum CoreError {
    #[error("{reason}")]
    Invalid { reason: String },
    #[error("Storage operation failed: {reason}")]
    Storage { reason: String },
    #[error("Access denied")]
    AccessDenied,
    #[error("The key is incorrect or the encrypted data was modified")]
    Authentication,
}

pub type Result<T> = std::result::Result<T, CoreError>;
pub fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid {
        reason: message.into(),
    }
}
impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Storage {
            reason: e.to_string(),
        }
    }
}
impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        invalid(format!("Invalid catalog or command: {e}"))
    }
}
impl From<zip::result::ZipError> for CoreError {
    fn from(e: zip::result::ZipError) -> Self {
        invalid(format!("Invalid archive: {e}"))
    }
}
