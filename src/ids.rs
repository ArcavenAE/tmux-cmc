use std::fmt;
use std::str::FromStr;

use crate::error::TmuxError;

/// A tmux session identifier (`$n`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub(crate) String);

/// A tmux window identifier (`@n`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(pub(crate) String);

/// A tmux pane identifier (`%n`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaneId(pub(crate) String);

// ── SessionId ────────────────────────────────────────────────────────────────

impl SessionId {
    /// Create from a raw `$n` string (validated).
    pub fn new(s: impl Into<String>) -> Result<Self, TmuxError> {
        let s = s.into();
        if s.starts_with('$') && s[1..].parse::<u64>().is_ok() {
            Ok(Self(s))
        } else {
            Err(TmuxError::InvalidId(s))
        }
    }

    pub(crate) fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SessionId {
    type Err = TmuxError;
    fn from_str(s: &str) -> Result<Self, TmuxError> {
        Self::new(s)
    }
}

// ── WindowId ─────────────────────────────────────────────────────────────────

impl WindowId {
    pub fn new(s: impl Into<String>) -> Result<Self, TmuxError> {
        let s = s.into();
        if s.starts_with('@') && s[1..].parse::<u64>().is_ok() {
            Ok(Self(s))
        } else {
            Err(TmuxError::InvalidId(s))
        }
    }

    pub(crate) fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for WindowId {
    type Err = TmuxError;
    fn from_str(s: &str) -> Result<Self, TmuxError> {
        Self::new(s)
    }
}

// ── PaneId ───────────────────────────────────────────────────────────────────

impl PaneId {
    pub fn new(s: impl Into<String>) -> Result<Self, TmuxError> {
        let s = s.into();
        if s.starts_with('%') && s[1..].parse::<u64>().is_ok() {
            Ok(Self(s))
        } else {
            Err(TmuxError::InvalidId(s))
        }
    }

    pub(crate) fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PaneId {
    type Err = TmuxError;
    fn from_str(s: &str) -> Result<Self, TmuxError> {
        Self::new(s)
    }
}
