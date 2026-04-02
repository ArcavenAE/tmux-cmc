use std::io::{BufRead, BufReader};
use std::process::ChildStdout;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::mpsc;

use crate::notification::Notification;
use crate::protocol::{Line, parse_line};
use crate::queue::PendingQueue;
use crate::response::Response;

/// Signals that the initial handshake block has been received.
pub struct HandshakeSignal {
    pub ready: Mutex<bool>,
    pub cv: Condvar,
}

impl HandshakeSignal {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ready: Mutex::new(false),
            cv: Condvar::new(),
        })
    }

    pub fn signal(&self) {
        let mut guard = self.ready.lock().expect("handshake lock poisoned");
        *guard = true;
        self.cv.notify_all();
    }
}

/// Reader thread entry point.
///
/// Reads lines from tmux stdout, demultiplexes response blocks and
/// notifications, and dispatches them to the appropriate channels.
#[allow(clippy::needless_pass_by_value)]
pub fn run(
    stdout: ChildStdout,
    queue: Arc<PendingQueue>,
    notif_senders: Arc<Mutex<Vec<mpsc::Sender<Notification>>>>,
    handshake: Arc<HandshakeSignal>,
) {
    let reader = BufReader::new(stdout);

    // State for the current in-flight response block
    let mut current_serial: Option<u64> = None;
    let mut current_flags: u32 = 0;
    let mut current_is_error = false;
    let mut accumulator: Vec<String> = Vec::new();
    let mut handshake_done = false;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break, // tmux exited or pipe broken
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
                    if serial == 0 && !handshake_done {
                        // Startup handshake complete
                        handshake_done = true;
                        handshake.signal();
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
    // Signal handshake in case connect() is still waiting (tmux exited immediately)
    if !handshake_done {
        handshake.signal();
    }

    let _ = current_is_error; // suppress unused warning
}

fn broadcast(senders: &Arc<Mutex<Vec<mpsc::Sender<Notification>>>>, notif: &Notification) {
    let mut guard = senders.lock().expect("notif senders lock poisoned");
    guard.retain(|tx| tx.send(notif.clone()).is_ok());
}
