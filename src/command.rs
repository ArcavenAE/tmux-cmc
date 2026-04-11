use crate::ids::{PaneId, SessionId, WindowId};

/// Options for creating a new session.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct NewSessionOptions {
    pub name: Option<String>,
    /// Start detached (don't attach the current client). Recommended.
    pub detached: bool,
    pub start_directory: Option<std::path::PathBuf>,
}

impl NewSessionOptions {
    /// Create new session options with the given name, detached.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            detached: true,
            ..Default::default()
        }
    }
}

/// Options for creating a new window.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct NewWindowOptions {
    pub session: SessionId,
    pub name: Option<String>,
    pub detached: bool,
    pub start_command: Option<String>,
}

/// Options for splitting a pane.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
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
    parts.push("-F '#{session_id}'".into());
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
    parts.push("-F '#{window_id}'".into());
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
    parts.push("-F '#{pane_id}'".into());
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
///
/// # Security
///
/// Control characters (`\n`, `\r`, `\0`) are stripped because they would
/// split the tmux command stream — each newline becomes a separate tmux
/// command, enabling injection of arbitrary tmux commands.
pub fn shell_escape(s: &str) -> String {
    // Strip control characters that could split the command stream.
    // Newline is the tmux command delimiter; \r and \0 are never valid
    // in tmux arguments.
    let sanitized: String = s
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\0'))
        .collect();

    if sanitized.is_empty() {
        return "''".to_owned();
    }
    // If the string has no special characters, pass through unquoted for readability
    if sanitized
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':'))
    {
        return sanitized;
    }
    // Single-quote the string, escaping internal single quotes as '\''
    format!("'{}'", sanitized.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "hello");
    }

    #[test]
    fn shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn shell_escape_with_quotes() {
        assert_eq!(shell_escape("it's"), r"'it'\''s'");
    }

    #[test]
    fn shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn shell_escape_strips_newline() {
        // After stripping \n, "foobar" is all alphanumeric → unquoted
        assert_eq!(shell_escape("foo\nbar"), "foobar");
    }

    #[test]
    fn shell_escape_strips_cr() {
        assert_eq!(shell_escape("foo\rbar"), "foobar");
    }

    #[test]
    fn shell_escape_strips_null() {
        assert_eq!(shell_escape("foo\0bar"), "foobar");
    }

    #[test]
    fn shell_escape_injection_attempt() {
        // Without newline stripping, this would send "kill-server" as a
        // separate tmux command. With stripping, it becomes one safe string.
        let result = shell_escape("foo\nkill-server");
        assert!(!result.contains('\n'), "newline must not survive escaping");
        assert_eq!(result, "fookill-server");
    }

    #[test]
    fn shell_escape_strips_newline_preserves_quoting() {
        // If the remaining string has special chars, it still gets quoted
        assert_eq!(shell_escape("foo bar\nbaz"), "'foo barbaz'");
    }

    #[test]
    fn shell_escape_only_control_chars() {
        assert_eq!(shell_escape("\n\r\0"), "''");
    }
}
