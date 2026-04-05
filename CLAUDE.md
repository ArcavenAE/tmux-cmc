# tmux-cmc — tmux Control Mode Client

## What This Is

A standalone Rust crate implementing the tmux control mode protocol (`tmux -CC`).
Bidirectional programmatic control of tmux: sessions, windows, panes, options,
send-keys, and a real-time notification stream. Persistent connection — no
per-command subprocess overhead.

Published at `ArcavenAE/tmux-cmc`. Used by aclaude for session management and
statusline push. Usable by anyone building tmux tooling in Rust.

## Build / Run / Test

Requires: Rust 1.85+ (Edition 2024), `just`, nightly rustfmt, tmux.

```sh
just build          # cargo build
just test           # cargo test (unit tests, no tmux required)
just test-integration # cargo test -- --include-ignored (requires live tmux)
just lint           # cargo clippy -- -D warnings
just fmt            # cargo +nightly fmt --all
just ci             # fmt-check + lint + build + test + deny
```

## Architecture

```
src/
  lib.rs            Public API re-exports, crate docs
  error.rs          TmuxError enum, Result<T>
  ids.rs            SessionId($n), WindowId(@n), PaneId(%n) newtypes
  protocol.rs       parse_line() → Line enum (stateless wire parser)
  queue.rs          PendingQueue: serial counter + BTreeMap<serial, oneshot::Sender>
  writer.rs         Writer thread: mpsc::Receiver → child stdin
  reader.rs         Reader thread: child stdout → queue.deliver() + notif broadcast
  connection.rs     Connection: owns Child, threads, PendingQueue, send_command()
  command.rs        Pure string builders for each tmux command
  response.rs       Response { serial, flags, output: Vec<String> }
  notification.rs   Notification enum (#[non_exhaustive])
  client.rs         Client = Arc<Connection>, all public methods
```

### Wire Protocol

Control mode is entered by spawning `tmux [-L <socket>] -CC`:
- Commands sent on stdin: one tmux command per line
- Responses on stdout: `%begin <ts> <serial> <flags>` … `%end <ts> <serial> <flags>`
- Errors: `%begin` … `%error <ts> <serial> <flags>`
- Async notifications arrive between responses: `%output`, `%pane-exited`, `%session-changed`, etc.
- Sessions `$n`, windows `@n`, panes `%n`

### Thread Model

```
  caller thread
    client.set_status_left(...)
    → connection.send_command("set-option ...")
      → queue.register() → (serial=N, rx)
      → writer_tx.send(PendingCommand { N, text })
      → rx.recv_timeout(30s)   ← blocks

  writer thread: PendingCommand → writeln! to tmux stdin

  reader thread: BufReader over stdout
    %begin ts N 0  → start accumulating for serial N
    %end ts N 0    → queue.deliver(N, response) → caller unblocks
    %output %1 ... → notif_tx broadcast
```

### Handshake

tmux emits `%begin 0 0 0` / `%end 0 0 0` on startup. `Client::connect()` blocks
on a `Condvar` until the reader thread sees this. Timeout → `TmuxError::HandshakeTimeout`.

## Public API

```rust
let client = Client::connect(&ConnectOptions { socket_name: Some("ac".into()), .. })?;

// Session
let session = client.new_session(&NewSessionOptions { name: Some("foo".into()), detached: true, .. })?;
client.has_session("foo")?;
client.kill_session(&session)?;

// Window / pane
let win = client.new_window(&NewWindowOptions { session: session.clone(), .. })?;
let pane = client.split_pane(&SplitPaneOptions { target: win, .. })?;
client.send_keys(&pane, "aclaude", false)?;

// Options
client.set_status_left(&session, " aclaude ")?;
client.set_status_right(&session, " ok ")?;
client.set_status_interval(&session, 2)?;
client.set_option(&OptionTarget::Global, "status", "on")?;

// Notifications
let rx = client.notifications();
while let Ok(notif) = rx.recv() {
    match notif {
        Notification::PaneExited { pane, exit_code } => { ... }
        _ => {}
    }
}

// Raw escape hatch
client.run_command("display-message -p '#{session_id}'")?;
```

## Platform Behavior

Key design decisions validated through cross-platform testing (macOS tmux
3.6a, Linux aarch64 tmux 3.5a). See finding-005 in aae-orc for the full
evidence trail.

- **Pty required for both stdin AND stdout.** tmux writes control protocol
  through the pty, not a separate stdout pipe.
- **Raw mode required** (cfmakeraw equivalent) to prevent echo and line
  discipline interference on Linux. macOS and Linux pty defaults differ.
- **Strip trailing `\r`** — raw mode disables OPOST, `\r\n` passes through
  unchanged, BufReader::lines() leaves `\r`.
- **FIFO response queue** — tmux assigns its own serial numbers, not the
  client's. First waiter gets first response.
- **DCS prefix** (`\x1bP1000p`) on first control mode line must be stripped.
- **Handshake detection** matches first `%begin`/`%end` pair regardless of
  serial (tmux 3.5a+ uses non-zero initial serial).
- **Format string quoting** — `-F #{session_id}` needs quoting in control
  mode: `-F '#{session_id}'`.

## Conventions

- **Language:** Rust. Entire codebase.
- **No unsafe code:** `#![forbid(unsafe_code)]` throughout.
- **No tokio:** Sync threads + `std::sync::mpsc` + `oneshot`. Async feature flag deferred.
- **Client is Clone:** `Arc<Connection>` — share freely across threads.
- **`TmuxError` is `#[non_exhaustive]`:** Callers must handle future variants.

## How to Work Here (kos Process)

Cross-repo questions belong in the orchestrator's `_kos/`. Questions scoped to
this crate live here.

### Session Protocol
1. Read the orchestrator `charter.md` (orient)
2. Identify the highest-value open question — or capture ideas in `_kos/ideas/`
3. Write an Exploration Brief in `_kos/probes/`
4. Do the probe work
5. Write a finding in `_kos/findings/`
6. Harvest: update affected nodes, move files if confidence changed
7. Update charter if bedrock changed

### Node Files
Nodes live in `_kos/nodes/[confidence]/[id].yaml`. Schema: kos v0.3.
