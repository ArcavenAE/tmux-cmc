//! `tmux-cmc` — tmux control mode client
//!
//! Bidirectional programmatic control of tmux via the control mode protocol
//! (`tmux -CC`). Exposes sessions, windows, panes, options, and an async
//! notification stream — all over a persistent connection with no per-command
//! subprocess overhead.
//!
//! # Quick start
//!
//! ```no_run
//! use tmux_cmc::{Client, ConnectOptions, NewSessionOptions};
//!
//! let client = Client::connect(&ConnectOptions::default())?;
//!
//! if !client.has_session("demo")? {
//!     let session = client.new_session(&NewSessionOptions {
//!         name: Some("demo".into()),
//!         detached: true,
//!         ..Default::default()
//!     })?;
//!     client.set_status_left(&session, "tmux-cmc demo")?;
//! }
//! # Ok::<(), tmux_cmc::TmuxError>(())
//! ```
#![forbid(unsafe_code)]

pub(crate) mod command;
pub(crate) mod connection;
pub(crate) mod ids;
pub(crate) mod notification;
pub(crate) mod protocol;
pub(crate) mod queue;
pub(crate) mod reader;
pub(crate) mod response;
pub(crate) mod writer;

pub mod client;
pub mod error;

pub use client::{Client, ConnectOptions, NewSessionOptions, NewWindowOptions, OptionTarget, SplitPaneOptions};
pub use error::TmuxError;
pub use ids::{PaneId, SessionId, WindowId};
pub use notification::Notification;
pub use response::Response;

pub type Result<T> = std::result::Result<T, TmuxError>;
