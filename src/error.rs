use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Idle(String),
    #[error("monitor not running")]
    NotRunning,
    #[error("monitor already running")]
    AlreadyRunning,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Clone, Serialize)]
pub struct LockPayload {
    pub locked: bool,
}

#[derive(Clone, Serialize)]
pub struct IdlePayload {
    pub idle: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct SuspendPayload {}

#[derive(Clone, Serialize)]
pub struct ResumePayload {}
