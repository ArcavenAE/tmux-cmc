use std::io::ErrorKind;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::error::{Result, TmuxError};
use crate::notification::Notification;
use crate::queue::{PendingCommand, PendingQueue};
use crate::reader::{HandshakeSignal, run as reader_run};
use crate::response::Response;
use crate::writer::run as writer_run;

const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Connection {
    /// Channel to send commands to the writer thread.
    writer_tx: mpsc::SyncSender<PendingCommand>,
    /// Pending command queue — maps serials to oneshot response channels.
    queue: Arc<PendingQueue>,
    /// Notification broadcast list.
    notif_senders: Arc<Mutex<Vec<mpsc::Sender<Notification>>>>,
    /// The tmux child process.
    _child: Mutex<Child>,
    /// Timeout for individual command responses.
    response_timeout: Duration,
}

impl Connection {
    /// Spawn `tmux -L <socket> -CC` and wait for the startup handshake.
    pub fn spawn(socket_name: Option<&str>, handshake_timeout: Duration) -> Result<Arc<Self>> {
        let mut cmd = Command::new("tmux");

        if let Some(name) = socket_name {
            cmd.args(["-L", name]);
        }

        // `-CC` enters control mode. `-x` would be exclusive.
        // `new-session -A -s tmux-cmc` attaches if exists, creates otherwise.
        cmd.args(["-CC", "new-session", "-A", "-D", "-s", "tmux-cmc-ctrl"]);

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                TmuxError::TmuxNotFound
            } else {
                TmuxError::Io(e)
            }
        })?;

        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");

        let queue = Arc::new(PendingQueue::new());
        let notif_senders: Arc<Mutex<Vec<mpsc::Sender<Notification>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let handshake = HandshakeSignal::new();

        // Spawn writer thread
        let (writer_tx, writer_rx) = mpsc::sync_channel::<PendingCommand>(64);
        {
            thread::Builder::new()
                .name("tmux-cmc-writer".into())
                .spawn(move || writer_run(writer_rx, stdin))
                .map_err(TmuxError::Io)?;
        }

        // Spawn reader thread
        {
            let queue_clone = Arc::clone(&queue);
            let notif_clone = Arc::clone(&notif_senders);
            let handshake_clone = Arc::clone(&handshake);
            thread::Builder::new()
                .name("tmux-cmc-reader".into())
                .spawn(move || reader_run(stdout, queue_clone, notif_clone, handshake_clone))
                .map_err(TmuxError::Io)?;
        }

        // Wait for handshake
        {
            let guard = handshake.ready.lock().expect("handshake lock poisoned");
            let (guard, timed_out) = handshake
                .cv
                .wait_timeout_while(guard, handshake_timeout, |ready| !*ready)
                .expect("handshake condvar poisoned");
            if timed_out.timed_out() && !*guard {
                return Err(TmuxError::HandshakeTimeout {
                    timeout: handshake_timeout,
                });
            }
        }

        Ok(Arc::new(Self {
            writer_tx,
            queue,
            notif_senders,
            _child: Mutex::new(child),
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
        }))
    }

    /// Send a raw tmux command and wait for the response.
    pub fn send_command(&self, text: impl Into<String>) -> Result<Response> {
        let text = text.into();
        let (serial, rx) = self.queue.register();

        self.writer_tx
            .send(PendingCommand { text: text.clone() })
            .map_err(|_| TmuxError::Disconnected)?;

        let response = rx
            .recv_timeout(self.response_timeout)
            .map_err(|_e| {
                // Could be timeout or disconnect — check if queue was drained
                TmuxError::ResponseTimeout { serial }
            })?;

        if response.is_error {
            Err(TmuxError::CommandError {
                serial,
                message: response.text(),
            })
        } else {
            Ok(response)
        }
    }

    /// Register a new notification receiver.
    pub fn subscribe(&self) -> mpsc::Receiver<Notification> {
        let (tx, rx) = mpsc::channel();
        self.notif_senders
            .lock()
            .expect("notif senders lock poisoned")
            .push(tx);
        rx
    }
}

/// Options for establishing a control mode connection.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub socket_name: Option<String>,
    pub handshake_timeout: Duration,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            socket_name: None,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
        }
    }
}
