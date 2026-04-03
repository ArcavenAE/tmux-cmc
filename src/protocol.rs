/// A parsed line from tmux control mode stdout.
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    /// `%begin <ts> <serial> <flags>`
    Begin { ts: u64, serial: u64, flags: u32 },
    /// `%end <ts> <serial> <flags>`
    End { ts: u64, serial: u64, flags: u32 },
    /// `%error <ts> <serial> <flags>`
    Error { ts: u64, serial: u64, flags: u32 },
    /// A notification line starting with `%` but not begin/end/error.
    Notification(RawNotification),
    /// An ordinary output line inside a %begin/%end block.
    Output(String),
}

/// A raw unparsed notification from tmux.
#[derive(Debug, PartialEq, Eq)]
pub struct RawNotification {
    pub kind: String,
    pub rest: String,
}

/// Parse a single line from tmux control mode output.
///
/// This function is stateless — it does not track whether we are inside a
/// `%begin`/`%end` block. The caller (reader thread) is responsible for
/// accumulation.
///
/// When using a pty (required for tmux 3.5a+), the first line may be wrapped
/// in a DCS (Device Control String) sequence: `\x1bP1000p%begin ...`. This
/// prefix is stripped before parsing.
pub fn parse_line(line: &str) -> Line {
    // Strip DCS prefix if present. tmux wraps the initial control mode output
    // in a DCS sequence: ESC P <params> <final-byte> <data>. The data starts
    // after the final byte (a lowercase letter). We look for '%' inside the
    // line and parse from there.
    let line = strip_dcs_prefix(line);

    if let Some(rest) = line.strip_prefix('%') {
        // Split into keyword and remainder
        let (keyword, remainder) = rest.split_once(' ').unwrap_or((rest, ""));

        match keyword {
            "begin" => {
                if let Some(tri) = parse_triple(remainder) {
                    return Line::Begin {
                        ts: tri.0,
                        serial: tri.1,
                        flags: tri.2,
                    };
                }
            }
            "end" => {
                if let Some(tri) = parse_triple(remainder) {
                    return Line::End {
                        ts: tri.0,
                        serial: tri.1,
                        flags: tri.2,
                    };
                }
            }
            "error" => {
                if let Some(tri) = parse_triple(remainder) {
                    return Line::Error {
                        ts: tri.0,
                        serial: tri.1,
                        flags: tri.2,
                    };
                }
            }
            _ => {}
        }

        Line::Notification(RawNotification {
            kind: keyword.to_owned(),
            rest: remainder.to_owned(),
        })
    } else {
        Line::Output(line.to_owned())
    }
}

/// Strip a DCS (Device Control String) prefix from a line.
///
/// tmux 3.5a+ wraps the initial control mode output in a DCS sequence:
/// `\x1bP1000p%begin ...`. This function strips the prefix so the parser
/// sees a clean `%begin` line.
///
/// If no DCS prefix is found, returns the original line unchanged.
fn strip_dcs_prefix(line: &str) -> &str {
    // DCS starts with ESC P (0x1b 0x50). Look for '%' after the DCS header.
    if line.starts_with('\x1b') {
        if let Some(pos) = line.find('%') {
            return &line[pos..];
        }
    }
    line
}

/// Parse three space-separated integers from a string.
fn parse_triple(s: &str) -> Option<(u64, u64, u32)> {
    let mut parts = s.split_ascii_whitespace();
    let a = parts.next()?.parse::<u64>().ok()?;
    let b = parts.next()?.parse::<u64>().ok()?;
    let c = parts.next()?.parse::<u32>().ok()?;
    Some((a, b, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_begin() {
        let line = parse_line("%begin 1712000000 1 0");
        assert_eq!(
            line,
            Line::Begin {
                ts: 1712000000,
                serial: 1,
                flags: 0
            }
        );
    }

    #[test]
    fn parses_end() {
        let line = parse_line("%end 1712000000 1 0");
        assert_eq!(
            line,
            Line::End {
                ts: 1712000000,
                serial: 1,
                flags: 0
            }
        );
    }

    #[test]
    fn parses_error() {
        let line = parse_line("%error 1712000000 2 0");
        assert_eq!(
            line,
            Line::Error {
                ts: 1712000000,
                serial: 2,
                flags: 0
            }
        );
    }

    #[test]
    fn parses_output_notification() {
        let line = parse_line("%output %1 hello world");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "output".into(),
                rest: "%1 hello world".into(),
            })
        );
    }

    #[test]
    fn parses_pane_exited() {
        let line = parse_line("%pane-exited %3 0");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "pane-exited".into(),
                rest: "%3 0".into(),
            })
        );
    }

    #[test]
    fn parses_session_changed() {
        let line = parse_line("%session-changed $1 my-session");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "session-changed".into(),
                rest: "$1 my-session".into(),
            })
        );
    }

    #[test]
    fn parses_window_add() {
        let line = parse_line("%window-add @2");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "window-add".into(),
                rest: "@2".into(),
            })
        );
    }

    #[test]
    fn parses_window_close() {
        let line = parse_line("%window-close @5");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "window-close".into(),
                rest: "@5".into(),
            })
        );
    }

    #[test]
    fn parses_sessions_changed() {
        let line = parse_line("%sessions-changed");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "sessions-changed".into(),
                rest: "".into(),
            })
        );
    }

    #[test]
    fn parses_exit() {
        let line = parse_line("%exit");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "exit".into(),
                rest: "".into(),
            })
        );
    }

    #[test]
    fn parses_unknown_notification() {
        let line = parse_line("%future-thing foo bar");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "future-thing".into(),
                rest: "foo bar".into(),
            })
        );
    }

    #[test]
    fn parses_ordinary_output() {
        let line = parse_line("$3");
        assert_eq!(line, Line::Output("$3".into()));
    }

    #[test]
    fn parses_empty_output() {
        let line = parse_line("");
        assert_eq!(line, Line::Output("".into()));
    }

    #[test]
    fn parses_output_with_percent_in_content() {
        // A line that starts with % but has a non-keyword word — treated as notification
        // with kind "bad" and rest "data"
        let line = parse_line("%bad data");
        assert_eq!(
            line,
            Line::Notification(RawNotification {
                kind: "bad".into(),
                rest: "data".into(),
            })
        );
    }

    #[test]
    fn begin_serial_zero_handshake() {
        // tmux emits %begin 0 0 0 / %end 0 0 0 on startup (pre-3.5a)
        let begin = parse_line("%begin 0 0 0");
        let end = parse_line("%end 0 0 0");
        assert_eq!(
            begin,
            Line::Begin {
                ts: 0,
                serial: 0,
                flags: 0
            }
        );
        assert_eq!(
            end,
            Line::End {
                ts: 0,
                serial: 0,
                flags: 0
            }
        );
    }

    #[test]
    fn strips_dcs_prefix_from_begin() {
        // tmux 3.5a+ wraps the initial line in a DCS sequence
        let line = parse_line("\x1bP1000p%begin 1775252542 271 0");
        assert_eq!(
            line,
            Line::Begin {
                ts: 1775252542,
                serial: 271,
                flags: 0
            }
        );
    }

    #[test]
    fn no_dcs_prefix_unchanged() {
        // Normal line without DCS prefix
        let line = parse_line("%end 1712000000 5 0");
        assert_eq!(
            line,
            Line::End {
                ts: 1712000000,
                serial: 5,
                flags: 0
            }
        );
    }
}
