use std::collections::BTreeMap;
use std::sync::Mutex;

use oneshot::Sender;

use crate::response::Response;

/// A pending command awaiting a response from the reader thread.
pub struct PendingCommand {
    pub text: String,
}

struct Inner {
    next_serial: u64,
    pending: BTreeMap<u64, Sender<Response>>,
}

/// Maps in-flight command serials to their oneshot response channels.
pub struct PendingQueue(Mutex<Inner>);

impl PendingQueue {
    pub fn new() -> Self {
        Self(Mutex::new(Inner {
            next_serial: 1, // 0 is reserved for the startup handshake
            pending: BTreeMap::new(),
        }))
    }

    /// Allocate a serial and register a response channel.
    /// Returns `(serial, receiver)`.
    pub fn register(&self) -> (u64, oneshot::Receiver<Response>) {
        let (tx, rx) = oneshot::channel();
        let mut inner = self.0.lock().expect("queue lock poisoned");
        let serial = inner.next_serial;
        inner.next_serial += 1;
        inner.pending.insert(serial, tx);
        (serial, rx)
    }

    /// Deliver a completed response to the waiting caller.
    /// Silently drops if no waiter (e.g. serial 0 handshake).
    pub fn deliver(&self, serial: u64, response: Response) {
        let mut inner = self.0.lock().expect("queue lock poisoned");
        if let Some(tx) = inner.pending.remove(&serial) {
            // Ignore send error — caller may have timed out and dropped rx
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
