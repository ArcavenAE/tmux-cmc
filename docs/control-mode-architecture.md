# tmux-cmc Control Mode Architecture

## How tmux Control Mode Works

tmux control mode (`tmux -CC`) is a client mode, not a server mode. A control
mode client is a regular tmux client that speaks a structured text protocol
instead of rendering a terminal UI. It attaches to a session like any other
client.

Key properties:
- A control mode client must be attached to a session (it needs a "home")
- One control mode client can manage ALL sessions on the same socket
- Commands sent via control mode operate on any session/window/pane by target
- Multiple control mode clients can coexist on the same socket
- Panes and windows move freely between sessions on the same socket
- Cross-socket operations are not supported (each socket is a separate server)

## Connection Patterns

### Pattern 1: Direct Attachment (no control session)

The control mode client attaches to the user's session directly.

```
socket "ac"
└── my-session (user session)
    ├── pane 0: user's shell
    └── clients: [control-mode, terminal]
```

```rust
// Create the session first
Command::new("tmux").args(["-L", "ac", "new-session", "-d", "-s", "my-session"]).status()?;

// Attach control mode to it
let client = Client::connect(&ConnectOptions {
    socket_name: Some("ac".into()),
    control_session_name: Some("my-session".into()),
    // control_session_command is ignored when attaching to existing session
    ..ConnectOptions::default()
});

// User attaches normally (no -d, both clients coexist)
// tmux -L ac attach-session -t my-session
```

**Pros:** Simplest. No extra sessions. No overhead.
**Cons:** If the session is killed, the control mode connection dies. Not
suitable for managing session lifecycle (shift change, rolling restart).
**Use when:** Single interactive session. The session outlives the control
need.

### Pattern 2: Dedicated Control Session (one per socket)

A lightweight control session runs an idle process (`cat`). The control mode
client attaches to it and manages all other sessions from there.

```
socket "ac"
├── _ctrl (control session, running cat)
│   └── clients: [control-mode]
├── my-session (user session)
│   └── clients: [terminal]
├── worker-1 (managed session)
└── worker-2 (managed session)
```

```rust
// Control session created automatically by connect()
let client = Client::connect(&ConnectOptions {
    socket_name: Some("ac".into()),
    control_session_name: Some("_ctrl".into()),
    control_session_command: Some("cat".into()),
    ..ConnectOptions::default()
});

// Create user sessions via control mode
client.new_session(&NewSessionOptions {
    name: Some("my-session".into()),
    detached: true,
    ..Default::default()
})?;
```

**Pros:** Survives session lifecycle. One connection manages everything.
Shift change doesn't break the control connection.
**Cons:** One extra session running `cat`. Visible in `tmux list-sessions`
(but can be filtered by naming convention like `_ctrl`).
**Use when:** Multi-session management. Agent orchestration. Shift change.
Any time sessions are created and destroyed while the controller stays alive.

### Pattern 3: Multiple Control Connections

Independent control mode clients for independent orchestrators. Each needs
a session to attach to.

```
socket "ac"
├── _ctrl-aclaude (aclaude's control session)
│   └── clients: [control-mode from aclaude]
├── _ctrl-marvel (marvel's control session)
│   └── clients: [control-mode from marvel]
├── my-session (user session)
├── team-alpha-worker-1 (marvel-managed)
└── team-alpha-worker-2 (marvel-managed)
```

```rust
// aclaude's connection
let aclaude_ctrl = Client::connect(&ConnectOptions {
    socket_name: Some("ac".into()),
    control_session_name: Some("_ctrl-aclaude".into()),
    control_session_command: Some("cat".into()),
    ..ConnectOptions::default()
});

// marvel's connection (separate process)
let marvel_ctrl = Client::connect(&ConnectOptions {
    socket_name: Some("ac".into()),
    control_session_name: Some("_ctrl-marvel".into()),
    control_session_command: Some("cat".into()),
    ..ConnectOptions::default()
});
```

**Pros:** Independent orchestrators don't interfere.
**Cons:** Multiple control sessions. Only needed when multiple independent
controllers exist.
**Use when:** aclaude and marvel both managing sessions on the same socket.
Rare — marvel would typically be the sole orchestrator.

## tmux Operations via Control Mode

All operations target sessions/windows/panes by name or ID, regardless of
which session the control mode client is attached to.

### Session lifecycle
```rust
// Create
let session = client.new_session(&NewSessionOptions {
    name: Some("worker-1".into()),
    detached: true,
    ..Default::default()
})?;

// Check
let exists = client.has_session("worker-1")?;

// Kill
client.kill_session(&session)?;
```

### Pane management
```rust
// Split a pane in any session
let pane = client.split_pane(&SplitPaneOptions {
    target: window_id,
    ..Default::default()
})?;

// Send keys to any pane
client.send_keys(&pane, "echo hello", false)?;

// Move pane between sessions (raw command)
client.run_command("join-pane -s worker-1:0.1 -t worker-2:0")?;
```

### Window management
```rust
// Move window between sessions (raw command)
client.run_command("move-window -s worker-1:0 -t worker-2:")?;
```

### Rolling restart (shift change)
```rust
// Create new session BEFORE killing old
let new = client.new_session(&NewSessionOptions {
    name: Some("worker-1-v2".into()),
    detached: true,
    ..Default::default()
})?;

// Optionally move panes from old to new
client.run_command("join-pane -s worker-1:0.0 -t worker-1-v2:0")?;

// Kill old
client.run_command("kill-session -t worker-1")?;
```

## Constraints

- **One socket = one tmux server.** All sessions on a socket share the same
  server process. Cross-socket operations are not possible.
- **Control mode client needs a session.** It must be attached to something.
  If that session is killed, the connection dies.
- **No `-d` on user attach.** If the user attaches with `tmux attach -t session -d`,
  it detaches ALL other clients including control mode. aclaude's attach
  does not use `-d`.
- **Notifications are global.** A control mode client receives notifications
  for ALL sessions on the socket, not just the one it's attached to. This
  is a feature for orchestration but means more noise to filter.

## Probed and Verified

- Control mode attaches to existing session: yes (macOS tmux 3.6a)
- One control mode manages multiple sessions: yes (macOS tmux 3.6a)
- Control mode survives other session lifecycle: yes (when on a separate session)
- Pane movement between sessions: yes (same socket)
- Window movement between sessions: yes (same socket)
- Cross-socket movement: no
- Multiple control mode clients on same socket: yes
- Terminal client + control mode client on same session: yes (without -d)

Needs verification on Linux tmux 3.5a for all of the above.
