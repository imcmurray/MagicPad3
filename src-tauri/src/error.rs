use serde::Serialize;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Platform backends use different variants per OS.
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("not supported on this platform: {0}")]
    Unsupported(String),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("permission denied: {0}")]
    Permission(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    #[allow(dead_code)]
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
