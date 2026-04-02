# tmux-cmc

tmux control mode client for Rust.

Bidirectional programmatic control of tmux via the control mode protocol (`tmux -CC`). Persistent connection, no per-command subprocess overhead, real-time notification stream.

## What this is

tmux has a native control mode protocol — run `tmux -CC` and tmux becomes a structured command interface: send commands on stdin, receive responses on stdout, and receive async notifications (pane output, exits, session changes) at any time.

This crate implements the client side of that protocol. You get a `Client` handle that speaks the wire format directly.

`tmux_interface` (the other Rust tmux crate) wraps the CLI with a new subprocess per command. This crate maintains a single persistent connection and routes responses and notifications by serial number.

## Usage

```toml
[dependencies]
tmux-cmc = "0.1"
```

```rust
use tmux_cmc::{Client, ConnectOptions, NewSessionOptions};

let client = Client::connect(&ConnectOptions::default())?;

if !client.has_session("my-session")? {
    let session = client.new_session(&NewSessionOptions {
        name: Some("my-session".into()),
        detached: true,
        ..Default::default()
    })?;

    client.set_status_left(&session, " my app ")?;
    client.set_status_right(&session, " connected ")?;
    client.set_status_interval(&session, 2)?;
}
```

## Wire protocol overview

Control mode is entered with `tmux [-L <socket>] -CC [command]`.

Every command gets a response block:

```
%begin <timestamp> <serial> <flags>
<output lines>
%end <timestamp> <serial> <flags>
```

Errors use `%error` instead of `%end`. Commands can be pipelined; responses are matched to callers by serial number.

Async notifications arrive between response blocks:

```
%output %1 hello world
%pane-exited %1 0
%session-changed $2 new-name
%sessions-changed
```

Entity identifiers: sessions `$n`, windows `@n`, panes `%n`.

## Architecture

Internally, `tmux-cmc` spawns `tmux -CC` as a child process and runs two threads:

- **Writer thread** — receives commands from callers via a channel, writes them to tmux stdin
- **Reader thread** — reads tmux stdout line by line, demultiplexes response blocks by serial, broadcasts notifications to registered receivers

All I/O is synchronous — no tokio required.

## Examples

```sh
cargo run --example session_start
cargo run --example status_push -- '$0'
```

## Requirements

- Rust 1.85+
- tmux installed in PATH

## Status

MVP. Implements session, window, and pane management, option control, key sending, and the notification stream. Covers the `aclaude session start` and statusline push use cases.

Not yet implemented: `list-sessions`, `list-panes`, reconnection on tmux exit, `async` feature flag.

## License

MIT
