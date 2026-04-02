use crate::ids::{PaneId, SessionId, WindowId};
use crate::protocol::RawNotification;

/// An asynchronous event emitted by tmux between command responses.
///
/// Delivered on the channel returned by [`Client::notifications`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Notification {
    /// Output was produced in a pane.
    Output { pane: PaneId, data: String },
    /// A pane's child process exited.
    PaneExited { pane: PaneId, exit_code: i32 },
    /// The attached session changed.
    SessionChanged { session: SessionId, name: String },
    /// A new window was added to the attached session.
    WindowAdded { window: WindowId },
    /// A window in the attached session was closed.
    WindowClosed { window: WindowId },
    /// Session list changed (session created or destroyed).
    SessionsChanged,
    /// tmux client is exiting (sent with `-CC`).
    Exit,
    /// A notification type not yet handled by this crate.
    Unhandled { raw: String },
}

impl Notification {
    /// Parse a [`RawNotification`] into a typed [`Notification`].
    pub fn from_raw(raw: &RawNotification) -> Self {
        match raw.kind.as_str() {
            "output" => parse_output(&raw.rest),
            "pane-exited" => parse_pane_exited(&raw.rest),
            "session-changed" => parse_session_changed(&raw.rest),
            "window-add" => parse_window_id_notif(&raw.rest, |w| Self::WindowAdded { window: w }),
            "window-close" => {
                parse_window_id_notif(&raw.rest, |w| Self::WindowClosed { window: w })
            }
            "sessions-changed" => Self::SessionsChanged,
            "exit" => Self::Exit,
            _ => Self::Unhandled {
                raw: format!("%{} {}", raw.kind, raw.rest),
            },
        }
    }
}

fn parse_output(rest: &str) -> Notification {
    // Format: `%<pane> <data>`
    if let Some((pane_str, data)) = rest.split_once(' ') {
        Notification::Output {
            pane: PaneId::new_unchecked(pane_str),
            data: unescape_output(data),
        }
    } else {
        // Pane with no output (rare but possible)
        Notification::Output {
            pane: PaneId::new_unchecked(rest),
            data: String::new(),
        }
    }
}

fn parse_pane_exited(rest: &str) -> Notification {
    let mut parts = rest.split_ascii_whitespace();
    let pane = PaneId::new_unchecked(parts.next().unwrap_or("%0"));
    let exit_code = parts
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    Notification::PaneExited { pane, exit_code }
}

fn parse_session_changed(rest: &str) -> Notification {
    let mut parts = rest.splitn(2, ' ');
    let session = SessionId::new_unchecked(parts.next().unwrap_or("$0"));
    let name = parts.next().unwrap_or("").to_owned();
    Notification::SessionChanged { session, name }
}

fn parse_window_id_notif(rest: &str, f: impl Fn(WindowId) -> Notification) -> Notification {
    let window = WindowId::new_unchecked(rest.split_ascii_whitespace().next().unwrap_or("@0"));
    f(window)
}

/// Unescape tmux control mode output escaping.
///
/// tmux escapes backslashes as `\134` and CR as `\015` in output notifications.
fn unescape_output(s: &str) -> String {
    // Fast path: no escapes present
    if !s.contains('\\') {
        return s.to_owned();
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // Read up to 3 octal digits
            let mut octal = String::with_capacity(3);
            for _ in 0..3 {
                match chars.peek() {
                    Some(&d) if d.is_ascii_digit() && d < '8' => {
                        octal.push(d);
                        chars.next();
                    }
                    _ => break,
                }
            }
            if octal.is_empty() {
                out.push('\\');
            } else if let Ok(n) = u32::from_str_radix(&octal, 8) {
                if let Some(c) = char::from_u32(n) {
                    out.push(c);
                } else {
                    out.push('\\');
                    out.push_str(&octal);
                }
            } else {
                out.push('\\');
                out.push_str(&octal);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_backslash() {
        assert_eq!(unescape_output("a\\134b"), "a\\b");
    }

    #[test]
    fn unescape_cr() {
        assert_eq!(unescape_output("a\\015b"), "a\rb");
    }

    #[test]
    fn unescape_no_escapes() {
        assert_eq!(unescape_output("hello world"), "hello world");
    }
}
