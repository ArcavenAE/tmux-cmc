use crate::ids::{PaneId, SessionId, WindowId};

/// Options for creating a new session.
#[derive(Debug, Default, Clone)]
pub struct NewSessionOptions {
    pub name: Option<String>,
    /// Start detached (don't attach the current client). Recommended.
    pub detached: bool,
    pub start_directory: Option<std::path::PathBuf>,
}

/// Options for creating a new window.
#[derive(Debug, Default, Clone)]
pub struct NewWindowOptions {
    pub session: SessionId,
    pub name: Option<String>,
    pub detached: bool,
    pub start_command: Option<String>,
}

/// Options for splitting a pane.
#[derive(Debug, Default, Clone)]
pub struct SplitPaneOptions {
    pub target: WindowId,
    /// `true` = vertical split (panes side by side), `false` = horizontal (stacked).
    pub vertical: bool,
    pub start_command: Option<String>,
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new_unchecked("$0")
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new_unchecked("@0")
    }
}

// ── Command string builders ───────────────────────────────────────────────────

/// `has-session -t <name>` — exits 0 if exists, 1 if not.
pub fn has_session(name: &str) -> String {
    format!("has-session -t {}", shell_escape(name))
}

/// `new-session` with the given options.
/// Returns the session id via `-P -F '#{session_id}'`.
pub fn new_session(opts: &NewSessionOptions) -> String {
    let mut parts = vec!["new-session".to_owned()];
    if opts.detached {
        parts.push("-d".into());
    }
    if let Some(name) = &opts.name {
        parts.push("-s".into());
        parts.push(shell_escape(name));
    }
    if let Some(dir) = &opts.start_directory {
        parts.push("-c".into());
        parts.push(shell_escape(&dir.to_string_lossy()));
    }
    // Print the session id on stdout
    parts.push("-P".into());
    parts.push("-F".into());
    parts.push("#{session_id}".into());
    parts.join(" ")
}

/// `kill-session -t <id>`
pub fn kill_session(id: &SessionId) -> String {
    format!("kill-session -t {id}")
}

/// `new-window` with options.
/// Returns the window id via `-P -F '#{window_id}'`.
pub fn new_window(opts: &NewWindowOptions) -> String {
    let mut parts = vec!["new-window".to_owned()];
    if opts.detached {
        parts.push("-d".into());
    }
    parts.push("-t".into());
    parts.push(opts.session.to_string());
    if let Some(name) = &opts.name {
        parts.push("-n".into());
        parts.push(shell_escape(name));
    }
    parts.push("-P".into());
    parts.push("-F".into());
    parts.push("#{window_id}".into());
    if let Some(cmd) = &opts.start_command {
        parts.push(shell_escape(cmd));
    }
    parts.join(" ")
}

/// `split-window` with options.
/// Returns the pane id via `-P -F '#{pane_id}'`.
pub fn split_pane(opts: &SplitPaneOptions) -> String {
    let mut parts = vec!["split-window".to_owned()];
    parts.push(if opts.vertical { "-h" } else { "-v" }.into());
    parts.push("-t".into());
    parts.push(opts.target.to_string());
    parts.push("-P".into());
    parts.push("-F".into());
    parts.push("#{pane_id}".into());
    if let Some(cmd) = &opts.start_command {
        parts.push(shell_escape(cmd));
    }
    parts.join(" ")
}

/// `kill-pane -t <id>`
pub fn kill_pane(id: &PaneId) -> String {
    format!("kill-pane -t {id}")
}

/// `send-keys -t <pane> [-l] <keys> Enter`
pub fn send_keys(pane: &PaneId, keys: &str, literal: bool) -> String {
    if literal {
        format!("send-keys -t {pane} -l {}", shell_escape(keys))
    } else {
        format!("send-keys -t {pane} {} Enter", shell_escape(keys))
    }
}

/// `set-option [-g] [-t <target>] <name> <value>`
pub fn set_option(target: &OptionTarget, name: &str, value: &str) -> String {
    match target {
        OptionTarget::Global => {
            format!(
                "set-option -g {} {}",
                shell_escape(name),
                shell_escape(value)
            )
        }
        OptionTarget::Session(id) => {
            format!(
                "set-option -t {id} {} {}",
                shell_escape(name),
                shell_escape(value)
            )
        }
        OptionTarget::Window(id) => {
            format!(
                "set-option -t {id} {} {}",
                shell_escape(name),
                shell_escape(value)
            )
        }
        OptionTarget::Pane(id) => {
            format!(
                "set-option -p -t {id} {} {}",
                shell_escape(name),
                shell_escape(value)
            )
        }
    }
}

/// Target for `set-option`.
#[derive(Debug, Clone)]
pub enum OptionTarget {
    Global,
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

/// Minimal shell escaping: wrap in single quotes, escaping any `'` inside.
///
/// Suitable for tmux command arguments that may contain spaces or special chars.
/// Does not handle binary data.
pub fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_owned();
    }
    // If the string has no special characters, pass through unquoted for readability
    if s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':'))
    {
        return s.to_owned();
    }
    // Single-quote the string, escaping internal single quotes as '\''
    format!("'{}'", s.replace('\'', r"'\''"))
}
