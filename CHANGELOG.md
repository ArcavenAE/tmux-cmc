# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

Initial release.

### Added

- `Client` — persistent tmux control mode connection via `tmux -CC`
- Session management: `new_session`, `has_session`, `kill_session`
- Window management: `new_window`
- Pane management: `split_pane`, `send_keys`
- Statusline control: `set_status_left`, `set_status_right`, `set_status_interval`, `set_status_enabled`
- Option management: `set_option` with session, window, and global targets
- Notification stream: `notifications()` returns a receiver for async events (pane exits, session changes, output, etc.)
- Raw command escape hatch: `run_command` for arbitrary tmux commands
- Typed IDs: `SessionId`, `WindowId`, `PaneId` with validation
- Thread-safe: `Client` is `Clone + Send + Sync`
- Zero unsafe code: `#![forbid(unsafe_code)]`
- Platform support: macOS (tmux 3.6a), Linux aarch64 (tmux 3.5a)

### Security

- Command injection prevention: `shell_escape` strips control characters (`\n`, `\r`, `\0`) that could split the tmux command stream
- Resource cleanup: `Drop` implementation kills child process and joins threads
