# tmux-cmc Roadmap — Ideas

## Framing

tmux-cmc is a Rust crate. Only Rust consumers can use it. marvel and switchboard
are Go — they are out of scope for this crate's feature development.

aclaude's primary use case: **session setup + statusline push**. Claude Code
owns the pane rendering. aclaude does not read pane output. The API surface
aclaude actually uses is narrow: connect, new_session, set_status_*, send_keys,
and potentially PaneExited.

Before adding any capability, ask: which consumer needs it, and is that consumer
using tmux-cmc? See probe-consumer-tmux-strategy for the full analysis.

---

## Immediate (unblock aclaude distribution)

- [ ] `cargo publish --dry-run` passes clean
- [ ] Semver policy decided (0.1.x patch, 0.x.0 breaking until 1.0)
- [ ] Release CI: push tag → cargo publish via GitHub Actions
- [ ] aclaude Cargo.toml migrates from path → git tag or crates.io

→ Nodes: question-crates-publish, question-aclaude-dep-source

---

## Short-term (validate what's actually needed)

**Probe: consumer requirements** (probe-consumer-tmux-strategy)
- [ ] Run the consumer probe — produce a finding with a capability matrix
- [ ] Confirm: does aclaude need PaneExited handling? Or is it a one-shot launcher?
- [ ] Confirm: is marvel's Go subprocess driver sufficient at realistic fleet scale?

**Integration test coverage**
- [ ] Integration tests run in CI against live tmux (ubuntu-latest has tmux)
- [ ] Cover every public Client method against a real tmux subprocess
- [ ] Test concurrent send_command() from multiple threads
- [ ] Test PaneExited delivery timing

→ Nodes: question-aclaude-pane-lifecycle, question-integration-tests

---

## Medium-term (fill confirmed gaps only)

**Notification coverage** — only add variants with a named consumer
- [ ] Audit which unhandled notification types aclaude actually needs
- [ ] Add structured handling for those specifically, not all 14+

**Query API** — only if a Rust consumer needs fleet inventory
- [ ] list-sessions, list-panes, display-message
- [ ] Not needed for aclaude's current use case; add when a consumer exists

**async feature flag** — only when there is a concrete async consumer
- [ ] tokio wrapper over the sync API
- [ ] No known consumer today → deferred

→ Nodes: question-notification-coverage, question-async-feature

---

## Out of scope for tmux-cmc

- **capture-pane**: aclaude doesn't need it (Claude Code owns the pane)
- **paste-buffer**: task injection is a protocol concern (director/FIFO), not tmux
- **marvel tmux integration**: marvel is Go, won't use tmux-cmc; see question-marvel-tmux-driver
- **switchboard tmux integration**: relay server, different relationship to tmux entirely
- **Go FFI (cgo)**: explicitly ruled out
