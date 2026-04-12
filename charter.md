# tmux-cmc Charter

Standalone Rust crate implementing the tmux control mode protocol (`tmux -CC`).
Bidirectional persistent connection — sync threads, FIFO response queue, zero unsafe.

Follows the kos process: Orient → Ideate → Question → Probe → Harvest → Promote.
Authoritative graph: `_kos/nodes/`.
Cross-repo questions belong in the orchestrator's charter.

Last updated: 2026-04-12 (session-016: charter created from orchestrator content migration)

---

## Bedrock

*Established. Evidence-based or decided with rationale.*

### B1: Core Protocol Verified on macOS and Linux

The control mode wire protocol implementation is verified on macOS (tmux 3.6a)
and Linux aarch64 (tmux 3.5a). Several platform-specific behaviors required
explicit handling — none were obvious from reading the protocol spec alone:

- **Pty required for both stdin AND stdout.** tmux writes control protocol
  through the pty, not a separate stdout pipe.
- **Raw mode required** (`cfmakeraw` equivalent) — macOS and Linux pty defaults
  differ. Without raw mode on Linux, echo and line discipline interfere.
- **Strip trailing `\r`** — raw mode disables OPOST; `\r\n` passes through
  unchanged; `BufReader::lines()` leaves `\r` on every line.
- **FIFO response queue** — tmux assigns its own serial numbers. First waiter
  gets first response, regardless of the serial the client sent.
- **DCS prefix** (`\x1bP1000p`) on first control mode line must be stripped.
- **Handshake detection** matches the first `%begin`/`%end` pair regardless
  of serial — tmux 3.5a+ uses a non-zero initial serial.
- **Format string quoting** — `-F #{session_id}` must be quoted in control
  mode: `-F '#{session_id}'`.

Graph: `_kos/nodes/bedrock/elem-wire-protocol.yaml`,
`_kos/nodes/bedrock/elem-sync-architecture.yaml`.
Evidence: aae-orc `_kos/findings/finding-005-*` (five fixes, cross-platform).
See also: aae-orc finding-001 (corrected).

### B2: Platform-Specific Behavior Requires Platform-Specific Testing

Code that interacts with OS-level APIs (pty, termios, file descriptors,
process spawning) may behave differently across platforms even when it
compiles for all of them. The macOS/Linux differences listed in B1 were
not caught by unit tests — all five were found through user-reported failures
on Linux, not by developer reasoning.

When a feature depends on platform-specific behavior and you cannot test
on the target platform:
- Add diagnostic output (e.g. `TMUX_CMC_DEBUG`) so the reporter can capture
  what's happening on their system.
- Do not declare fixes based on reasoning from a different platform.
- The reporter's diagnostic trace is more reliable than the developer's theory.

Evidence: aae-orc finding-005 — five fixes required; developer's theories
wrong four times; reporter's debug trace identified the root cause each time.

---

## Frontier

*Actively open. Expected to resolve through probes or design work.*

### F1: No Integration Tests Against Live tmux on Linux

Unit tests cover the protocol parser (15 tests, no tmux required). Integration
tests are marked `#[ignore]` and require a live tmux session — they exist but
are thin. All platform bugs to date were found by a user, not by tests.

What's needed before the crate can be considered production-ready:
- Every public `Client` method tested against a real tmux subprocess.
- Graceful handling of tmux subprocess death mid-command.
- Notification delivery timing (`%output` arrival relative to `%end`).
- Concurrent `send_command()` calls from multiple threads.
- Handshake timeout behavior.
- CI running integration tests on both `ubuntu-latest` and `macos-latest`
  (both have tmux available on GitHub-hosted runners — this is solvable).

Open design question: is a mock tmux subprocess (a small Rust binary speaking
the wire protocol) worth building for deterministic testing? Or is a live
tmux against a real subprocess sufficient?

Graph: `_kos/nodes/frontier/question-integration-tests.yaml`,
`_kos/nodes/frontier/question-platform-matrix.yaml`.
Evidence: aae-orc B13, F20 (tmux-cmc and forestage session maturity).

---

## Graveyard

*No entries yet.*
