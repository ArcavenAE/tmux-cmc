use std::sync::{Arc, mpsc};

use crate::command;
use crate::connection::Connection;
use crate::error::{Result, TmuxError};
use crate::ids::{PaneId, SessionId, WindowId};
use crate::notification::Notification;
use crate::response::Response;

pub use crate::command::{NewSessionOptions, NewWindowOptions, OptionTarget, SplitPaneOptions};
pub use crate::connection::ConnectOptions;

/// A handle to a tmux control mode connection.
///
/// `Client` is cheap to clone — all clones share the same underlying connection.
#[derive(Clone)]
pub struct Client {
    conn: Arc<Connection>,
}

impl Client {
    /// Connect to tmux via control mode.
    ///
    /// Spawns `tmux [-L <socket>] -CC new-session -A -D -s tmux-cmc-ctrl` as a
    /// child process and waits for the startup handshake.
    pub fn connect(opts: &ConnectOptions) -> Result<Self> {
        let conn = Connection::spawn(opts.socket_name.as_deref(), opts.handshake_timeout)?;
        Ok(Self { conn })
    }

    /// Subscribe to async notifications from tmux.
    ///
    /// Each call returns an independent receiver. The underlying channel is
    /// closed when the tmux connection is lost.
    pub fn notifications(&self) -> mpsc::Receiver<Notification> {
        self.conn.subscribe()
    }

    // ── Session ───────────────────────────────────────────────────────────────

    /// Check if a session with the given name exists.
    pub fn has_session(&self, name: &str) -> Result<bool> {
        match self.conn.send_command(command::has_session(name)) {
            Ok(_) => Ok(true),
            Err(TmuxError::CommandError { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Create a new session. Returns the new session's ID.
    pub fn new_session(&self, opts: &NewSessionOptions) -> Result<SessionId> {
        let resp = self.conn.send_command(command::new_session(opts))?;
        let id_str = resp
            .first_line()
            .ok_or_else(|| TmuxError::UnexpectedResponse("empty new-session output".into()))?;
        SessionId::new(id_str.trim())
            .map_err(|_| TmuxError::UnexpectedResponse(format!("invalid session id: {id_str}")))
    }

    /// Kill (destroy) a session.
    pub fn kill_session(&self, id: &SessionId) -> Result<()> {
        self.conn.send_command(command::kill_session(id))?;
        Ok(())
    }

    // ── Window ────────────────────────────────────────────────────────────────

    /// Create a new window. Returns the new window's ID.
    pub fn new_window(&self, opts: &NewWindowOptions) -> Result<WindowId> {
        let resp = self.conn.send_command(command::new_window(opts))?;
        let id_str = resp
            .first_line()
            .ok_or_else(|| TmuxError::UnexpectedResponse("empty new-window output".into()))?;
        WindowId::new(id_str.trim())
            .map_err(|_| TmuxError::UnexpectedResponse(format!("invalid window id: {id_str}")))
    }

    // ── Pane ──────────────────────────────────────────────────────────────────

    /// Split a pane. Returns the new pane's ID.
    pub fn split_pane(&self, opts: &SplitPaneOptions) -> Result<PaneId> {
        let resp = self.conn.send_command(command::split_pane(opts))?;
        let id_str = resp
            .first_line()
            .ok_or_else(|| TmuxError::UnexpectedResponse("empty split-window output".into()))?;
        PaneId::new(id_str.trim())
            .map_err(|_| TmuxError::UnexpectedResponse(format!("invalid pane id: {id_str}")))
    }

    /// Kill (close) a pane.
    pub fn kill_pane(&self, id: &PaneId) -> Result<()> {
        self.conn.send_command(command::kill_pane(id))?;
        Ok(())
    }

    // ── Input ─────────────────────────────────────────────────────────────────

    /// Send keys to a pane.
    ///
    /// If `literal` is true, keys are sent without interpretation (no Enter appended).
    /// If false, keys are sent as a command line followed by Enter.
    pub fn send_keys(&self, pane: &PaneId, keys: &str, literal: bool) -> Result<()> {
        self.conn
            .send_command(command::send_keys(pane, keys, literal))?;
        Ok(())
    }

    // ── Options ───────────────────────────────────────────────────────────────

    /// Set a tmux option at the specified target scope.
    pub fn set_option(&self, target: &OptionTarget, name: &str, value: &str) -> Result<()> {
        self.conn
            .send_command(command::set_option(target, name, value))?;
        Ok(())
    }

    /// Set a global tmux option (`set-option -g`).
    pub fn set_global_option(&self, name: &str, value: &str) -> Result<()> {
        self.set_option(&OptionTarget::Global, name, value)
    }

    // ── Statusline convenience ────────────────────────────────────────────────

    /// Set the left statusline for a session.
    pub fn set_status_left(&self, session: &SessionId, content: &str) -> Result<()> {
        self.set_option(
            &OptionTarget::Session(session.clone()),
            "status-left",
            content,
        )
    }

    /// Set the right statusline for a session.
    pub fn set_status_right(&self, session: &SessionId, content: &str) -> Result<()> {
        self.set_option(
            &OptionTarget::Session(session.clone()),
            "status-right",
            content,
        )
    }

    /// Set the statusline refresh interval (seconds) for a session.
    pub fn set_status_interval(&self, session: &SessionId, secs: u32) -> Result<()> {
        self.set_option(
            &OptionTarget::Session(session.clone()),
            "status-interval",
            &secs.to_string(),
        )
    }

    /// Enable or disable the status bar for a session.
    pub fn set_status_enabled(&self, session: &SessionId, enabled: bool) -> Result<()> {
        self.set_option(
            &OptionTarget::Session(session.clone()),
            "status",
            if enabled { "on" } else { "off" },
        )
    }

    // ── Escape hatch ──────────────────────────────────────────────────────────

    /// Send a raw tmux command string and return the response.
    pub fn run_command(&self, cmd: &str) -> Result<Response> {
        self.conn.send_command(cmd)
    }
}
