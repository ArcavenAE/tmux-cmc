use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};

use crate::notification::Notification;
use crate::protocol::{Line, parse_line};
use crate::queue::PendingQueue;
use crate::response::Response;

/// State of the startup handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    /// Waiting for the initial `%begin 0 0 0` / `%end 0 0 0` block.
    Waiting,
    /// Handshake completed successfully.
    Ready,
    /// tmux exited before completing the handshake.
    Failed,
}

/// Signals the outcome of the startup handshake to the spawning thread.
pub struct HandshakeSignal {
    pub state: Mutex<HandshakeState>,
    pub cv: Condvar,
}

impl HandshakeSignal {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(HandshakeState::Waiting),
            cv: Condvar::new(),
        })
    }

    pub fn signal_ready(&self) {
        let mut guard = self.state.lock().expect("handshake lock poisoned");
        *guard = HandshakeState::Ready;
        self.cv.notify_all();
    }

    pub fn signal_failed(&self) {
        let mut guard = self.state.lock().expect("handshake lock poisoned");
        *guard = HandshakeState::Failed;
        self.cv.notify_all();
    }
}

/// Reader thread entry point.
///
/// Reads lines from the tmux control protocol stream (either a pty primary or
/// a piped stdout), demultiplexes response blocks and notifications, and
/// dispatches them to the appropriate channels.
#[allow(clippy::needless_pass_by_value)]
pub fn run(
    source: impl Read,
    queue: Arc<PendingQueue>,
    notif_senders: Arc<Mutex<Vec<mpsc::Sender<Notification>>>>,
    handshake: Arc<HandshakeSignal>,
) {
    let reader = BufReader::new(source);

    // State for the current in-flight response block
    let mut current_serial: Option<u64> = None;
    let mut current_flags: u32 = 0;
    let mut current_is_error = false;
    let mut accumulator: Vec<String> = Vec::new();
    let mut handshake_done = false;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break, // tmux exited or pty closed
        };

        match parse_line(&line) {
            Line::Begin { serial, flags, .. } => {
                current_serial = Some(serial);
                current_flags = flags;
                current_is_error = false;
                accumulator.clear();
            }

            Line::End { serial, .. } => {
                if current_serial == Some(serial) {
                    if !handshake_done {
                        // First %begin/%end pair = startup handshake complete.
                        // tmux 3.5a+ may use a non-zero serial for the initial block.
                        handshake_done = true;
                        handshake.signal_ready();
                    } else {
                        let response = Response {
                            serial,
                            flags: current_flags,
                            output: std::mem::take(&mut accumulator),
                            is_error: false,
                        };
                        queue.deliver(serial, response);
                    }
                    current_serial = None;
                }
            }

            Line::Error { serial, .. } => {
                if current_serial == Some(serial) {
                    let response = Response {
                        serial,
                        flags: current_flags,
                        output: std::mem::take(&mut accumulator),
                        is_error: true,
                    };
                    queue.deliver(serial, response);
                    current_serial = None;
                    current_is_error = false;
                }
            }

            Line::Notification(raw) => {
                let is_exit = raw.kind == "exit";
                let notif = Notification::from_raw(&raw);
                broadcast(&notif_senders, &notif);
                if is_exit {
                    break;
                }
            }

            Line::Output(text) => {
                if current_serial.is_some() {
                    accumulator.push(text);
                }
                // Lines outside a %begin/%end block are discarded
            }
        }
    }

    // tmux exited — drain the queue so all waiting callers get Disconnected
    queue.drain();
    // Signal failure if tmux exited before completing the handshake
    if !handshake_done {
        handshake.signal_failed();
    }

    let _ = current_is_error; // suppress unused warning
}

fn broadcast(senders: &Arc<Mutex<Vec<mpsc::Sender<Notification>>>>, notif: &Notification) {
    let mut guard = senders.lock().expect("notif senders lock poisoned");
    guard.retain(|tx| tx.send(notif.clone()).is_ok());
}
