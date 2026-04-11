//! Start a tmux session, configure the statusline, and send a command.
//!
//! Usage: `cargo run --example session_start`
//!
//! Requires tmux to be installed.

use tmux_cmc::{Client, ConnectOptions, NewSessionOptions};

fn main() -> tmux_cmc::Result<()> {
    let client = Client::connect(&ConnectOptions::with_socket("example"))?;

    println!("Connected to tmux control mode.");

    let session_name = "tmux-cmc-example";

    let session = if client.has_session(session_name)? {
        println!("Session '{session_name}' already exists.");
        client.new_session(&NewSessionOptions::named(session_name))?
    } else {
        println!("Creating session '{session_name}'...");
        client.new_session(&NewSessionOptions::named(session_name))?
    };

    println!("Session id: {session}");

    // Configure statusline
    client.set_status_enabled(&session, true)?;
    client.set_status_interval(&session, 2)?;
    client.set_status_left(&session, " tmux-cmc ")?;
    client.set_status_right(&session, " connected ")?;

    println!("Statusline configured.");
    println!("Attach with: tmux -L example attach-session -t {session_name}");

    Ok(())
}
