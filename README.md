# tmux-cmc

[![Crates.io](https://img.shields.io/crates/v/tmux-cmc.svg)](https://crates.io/crates/tmux-cmc)
[![Docs.rs](https://docs.rs/tmux-cmc/badge.svg)](https://docs.rs/tmux-cmc)
[![CI](https://github.com/ArcavenAE/tmux-cmc/actions/workflows/ci.yml/badge.svg)](https://github.com/ArcavenAE/tmux-cmc/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://github.com/ArcavenAE/tmux-cmc)

tmux control mode client for Rust.

Bidirectional programmatic control of tmux via the control mode protocol (`tmux -CC`). Persistent connection, no per-command subprocess overhead, real-time notification stream.

## What this is

tmux has a native control mode protocol — run `tmux -CC` and tmux becomes a structured command interface: send commands on stdin, receive responses on stdout, and receive async notifications (pane output, exits, session changes) at any time.

This crate implements the client side of that protocol. You get a `Client` handle that speaks the wire format directly.

`tmux_interface` (the other Rust tmux crate) wraps the CLI with a new subprocess per command. This crate maintains a single persistent connection and routes responses and notifications by serial number.

## Why this exists

tmux-cmc was built for [forestage](https://github.com/ArcavenAE/forestage) (a BYOA agent CLI) and [marvel](https://github.com/ArcavenAE/marvel) (an agent fleet control plane). forestage uses it for session management and real-time statusline push. marvel will use it for pane lifecycle events in multi-agent tmux layouts. The crate is independently useful — it's published separately because anyone building tmux tooling in Rust needs the same protocol implementation.

## Usage

```toml
[dependencies]
tmux-cmc = "0.1"
```

```rust
use tmux_cmc::{Client, ConnectOptions, NewSessionOptions};

let client = Client::connect(&ConnectOptions::default())?;

if !client.has_session("my-session")? {
    let session = client.new_session(&NewSessionOptions::named("my-session"))?;

    client.set_status_left(&session, " my app ")?;
    client.set_status_right(&session, " connected ")?;
    client.set_status_interval(&session, 2)?;
}
# Ok::<(), tmux_cmc::TmuxError>(())
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

All I/O is synchronous — no tokio required. `Client` is `Clone` (cheap, via `Arc`) and safe to share across threads.

When the `Client` is dropped, the tmux child process is killed and threads are joined.

## Platform support

Unix only (macOS, Linux). Tested on macOS (tmux 3.6a) and Linux aarch64 (tmux 3.5a). tmux is a unix tool — this crate will not compile on non-unix platforms.

## Examples

```sh
cargo run --example session_start
cargo run --example status_push -- '$0'
```

## Requirements

- Rust 1.85+ (Edition 2024)
- tmux installed in PATH

## Status

Pre-1.0. Implements session, window, and pane management, option control, key sending, and the notification stream.

Not yet implemented: `list-sessions`, `list-panes`, reconnection on tmux exit, `async` feature flag.

## License

MIT
