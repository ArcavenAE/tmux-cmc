//! Push statusline updates to a running tmux session in real time.
//!
//! Usage: `cargo run --example status_push -- <session-name>`
//!
//! Requires an existing tmux session. Pushes a timestamp update every second
//! for 10 seconds, then exits.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tmux_cmc::{Client, ConnectOptions, SessionId};

fn main() -> tmux_cmc::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let session_id_str = args.get(1).map(String::as_str).unwrap_or("$0");

    let session = SessionId::new(session_id_str).unwrap_or_else(|_| {
        eprintln!("Pass a session id like $0 as the first argument.");
        std::process::exit(1);
    });

    let client = Client::connect(&ConnectOptions::default())?;
    println!("Connected. Pushing statusline updates to {session} for 10 seconds...");

    for i in 0..10 {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let left = format!(" tmux-cmc | tick {i} ");
        let right = format!(" {ts} ");

        client.set_status_left(&session, &left)?;
        client.set_status_right(&session, &right)?;

        println!("  [{i}] pushed: left='{left}' right='{right}'");
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("Done.");
    Ok(())
}
