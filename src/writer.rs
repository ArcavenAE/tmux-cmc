use std::io::{BufWriter, Write};
use std::sync::mpsc;

use crate::queue::PendingCommand;

/// Writer thread entry point.
///
/// Receives `PendingCommand` values from the command channel and writes them
/// to tmux's stdin. Single-threaded to preserve command ordering.
///
/// Accepts any `Write` implementor — either a `ChildStdin` (pipe) or a `File`
/// (pty primary).
pub fn run(rx: mpsc::Receiver<PendingCommand>, writer: impl Write) {
    let mut writer = BufWriter::new(writer);
    for cmd in rx {
        // Write the command text followed by a newline
        if writeln!(writer, "{}", cmd.text).is_err() {
            // stdin closed — tmux exited; drain remaining commands silently
            break;
        }
        if writer.flush().is_err() {
            break;
        }
    }
}
