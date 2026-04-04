use std::collections::VecDeque;
use std::sync::Mutex;

use oneshot::Sender;

use crate::response::Response;

/// A pending command awaiting a response from the reader thread.
pub struct PendingCommand {
    pub text: String,
}

struct Inner {
    pending: VecDeque<Sender<Response>>,
}

/// FIFO queue mapping in-flight commands to their response channels.
///
/// tmux assigns its own serial numbers to responses — they do not match
/// the client's internal numbering. Commands are processed in order, so
/// the first pending waiter receives the first response regardless of
/// the serial tmux assigns.
pub struct PendingQueue(Mutex<Inner>);

impl PendingQueue {
    pub fn new() -> Self {
        Self(Mutex::new(Inner {
            pending: VecDeque::new(),
        }))
    }

    /// Register a response channel for the next command.
    /// Returns a receiver that will get the response.
    pub fn register(&self) -> oneshot::Receiver<Response> {
        let (tx, rx) = oneshot::channel();
        let mut inner = self.0.lock().expect("queue lock poisoned");
        inner.pending.push_back(tx);
        rx
    }

    /// Deliver a completed response to the next waiting caller (FIFO).
    /// Silently drops if no waiter.
    pub fn deliver(&self, response: Response) {
        let mut inner = self.0.lock().expect("queue lock poisoned");
        if let Some(tx) = inner.pending.pop_front() {
            let _ = tx.send(response);
        }
    }

    /// Drop all pending senders, causing all waiting receivers to observe disconnect.
    pub fn drain(&self) {
        let mut inner = self.0.lock().expect("queue lock poisoned");
        inner.pending.clear();
    }
}

impl Default for PendingQueue {
    fn default() -> Self {
        Self::new()
    }
}
