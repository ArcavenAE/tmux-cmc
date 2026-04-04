use std::fs::File;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{self, OptionalActions};

use crate::error::{Result, TmuxError};
use crate::notification::Notification;
use crate::queue::{PendingCommand, PendingQueue};
use crate::reader::{HandshakeSignal, HandshakeState, run as reader_run};
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

/// Create a pty pair and return (primary_file, stdin_stdio, stdout_stdio).
///
/// tmux in control mode requires a pty — it calls `tcgetattr` on stdin and
/// writes control protocol output through the same pty rather than to a
/// separate stdout pipe. Both stdin and stdout use the pty secondary;
/// we read and write through the primary.
fn create_pty_pair() -> std::result::Result<(File, Stdio, Stdio), std::io::Error> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    let primary = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).map_err(std::io::Error::from)?;
    grantpt(&primary).map_err(std::io::Error::from)?;
    unlockpt(&primary).map_err(std::io::Error::from)?;

    let secondary_name = ptsname(&primary, Vec::new()).map_err(std::io::Error::from)?;
    let secondary_path = Path::new(OsStr::from_bytes(secondary_name.as_bytes()));

    // Open secondary twice — separate fds for stdin and stdout
    let secondary_in = File::options()
        .read(true)
        .write(true)
        .open(secondary_path)?;
    let secondary_out = File::options()
        .read(true)
        .write(true)
        .open(secondary_path)?;

    // Set the pty to raw mode. Without this, the kernel's terminal line
    // discipline echoes input back to the primary and may process special
    // characters in the control protocol output. Raw mode passes bytes
    // through unchanged in both directions.
    let mut attrs = termios::tcgetattr(&secondary_in).map_err(std::io::Error::from)?;
    // Equivalent to cfmakeraw() — disable terminal processing so the
    // control protocol passes through unchanged.
    use rustix::termios::{InputModes, LocalModes, OutputModes};
    attrs.input_modes &= !(InputModes::BRKINT
        | InputModes::ICRNL
        | InputModes::IGNBRK
        | InputModes::IGNCR
        | InputModes::INLCR
        | InputModes::ISTRIP
        | InputModes::IXON
        | InputModes::PARMRK);
    attrs.output_modes &= !OutputModes::OPOST;
    attrs.local_modes &= !(LocalModes::ECHO
        | LocalModes::ECHONL
        | LocalModes::ICANON
        | LocalModes::IEXTEN
        | LocalModes::ISIG);
    termios::tcsetattr(&secondary_in, OptionalActions::Now, &attrs)
        .map_err(std::io::Error::from)?;

    let primary_file = File::from(primary);

    Ok((primary_file, Stdio::from(secondary_in), Stdio::from(secondary_out)))
}

impl Connection {
    /// Spawn `tmux -L <socket> -CC` and wait for the startup handshake.
    pub fn spawn(socket_name: Option<&str>, handshake_timeout: Duration) -> Result<Arc<Self>> {
        let mut cmd = Command::new("tmux");

        if let Some(name) = socket_name {
            cmd.args(["-L", name]);
        }

        // `-CC` enters control mode.
        // `new-session -A -s tmux-cmc` attaches if exists, creates otherwise.
        cmd.args(["-CC", "new-session", "-A", "-D", "-s", "tmux-cmc-ctrl"]);

        // Use a pty for both stdin and stdout — tmux calls tcgetattr on stdin
        // and writes control protocol output through the same pty, not to a
        // separate stdout pipe.
        let (primary_file, stdin_stdio, stdout_stdio) = create_pty_pair()?;


        cmd.stdin(stdin_stdio)
            .stdout(stdout_stdio)
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TmuxError::TmuxNotFound
            } else {
                TmuxError::Io(e)
            }
        })?;

        let stderr_handle = child.stderr.take();

        // Clone primary for reading — writer and reader share the same pty fd.
        let primary_reader = primary_file
            .try_clone()
            .map_err(TmuxError::Io)?;

        let queue = Arc::new(PendingQueue::new());
        let notif_senders: Arc<Mutex<Vec<mpsc::Sender<Notification>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let handshake = HandshakeSignal::new();

        // Spawn writer thread — writes commands to the pty primary
        let (writer_tx, writer_rx) = mpsc::sync_channel::<PendingCommand>(64);
        {
            thread::Builder::new()
                .name("tmux-cmc-writer".into())
                .spawn(move || writer_run(writer_rx, primary_file))
                .map_err(TmuxError::Io)?;
        }

        // Spawn reader thread — reads control protocol from the pty primary
        {
            let queue_clone = Arc::clone(&queue);
            let notif_clone = Arc::clone(&notif_senders);
            let handshake_clone = Arc::clone(&handshake);
            thread::Builder::new()
                .name("tmux-cmc-reader".into())
                .spawn(move || reader_run(primary_reader, queue_clone, notif_clone, handshake_clone))
                .map_err(TmuxError::Io)?;
        }

        // Wait for handshake
        let state = {
            let guard = handshake.state.lock().expect("handshake lock poisoned");
            let (guard, _timed_out) = handshake
                .cv
                .wait_timeout_while(guard, handshake_timeout, |s| {
                    *s == HandshakeState::Waiting
                })
                .expect("handshake condvar poisoned");
            *guard
        };

        match state {
            HandshakeState::Ready => {}
            HandshakeState::Failed => {
                let stderr = stderr_handle
                    .map(|mut h| {
                        let mut buf = String::new();
                        let _ = h.read_to_string(&mut buf);
                        buf
                    })
                    .unwrap_or_default();
                return Err(TmuxError::StartupFailed {
                    stderr: if stderr.trim().is_empty() {
                        "tmux exited immediately with no error output".into()
                    } else {
                        stderr.trim().to_string()
                    },
                });
            }
            HandshakeState::Waiting => {
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
        let rx = self.queue.register();

        self.writer_tx
            .send(PendingCommand { text: text.clone() })
            .map_err(|_| TmuxError::Disconnected)?;

        let response = rx
            .recv_timeout(self.response_timeout)
            .map_err(|_e| TmuxError::ResponseTimeout { serial: 0 })?;

        if response.is_error {
            Err(TmuxError::CommandError {
                serial: response.serial,
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
