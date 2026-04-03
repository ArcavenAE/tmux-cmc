use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TmuxError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tmux not found — is tmux installed and in PATH?")]
    TmuxNotFound,

    #[error("control mode handshake timed out after {timeout:?}")]
    HandshakeTimeout { timeout: Duration },

    #[error("tmux exited during startup: {stderr}")]
    StartupFailed { stderr: String },

    #[error("tmux command error (serial {serial}): {message}")]
    CommandError { serial: u64, message: String },

    #[error("connection to tmux lost")]
    Disconnected,

    #[error("response parse error: {0}")]
    ParseError(String),

    #[error("timeout waiting for response (serial {serial})")]
    ResponseTimeout { serial: u64 },

    #[error("unexpected response format: {0}")]
    UnexpectedResponse(String),

    #[error("invalid entity id: {0}")]
    InvalidId(String),
}

pub type Result<T> = std::result::Result<T, TmuxError>;
