# tmux-cmc as the Rust ecosystem's tmux control mode crate

*Idea — pre-hypothesis brainstorming. Generative, uncommitted.*

## The observation

No viable Rust crate exists for tmux control mode (-CC). tmux_interface
wraps the CLI with a new subprocess per command — no persistent connection,
no real-time notification stream, no control mode support. tmux-cmc fills
a real gap in the Rust ecosystem.

The crate is already working: bidirectional protocol, sync threads, FIFO
response queue, 18+ tests, verified on macOS (tmux 3.6a) and Linux aarch64
(tmux 3.5a). Two consumers: forestage (session management, statusline) and
potentially marvel (pane lifecycle events).

## What wider use requires

1. **Publish to crates.io** — question-crates-publish. Needs: Cargo.toml
   metadata, crate-level docs, examples, release CI with version tags.

2. **API stability audit** — question-api-stability. The public API
   (Client, ConnectOptions, NewSessionOptions, etc.) hasn't been designed
   for external consumers. Breaking changes are cheap today; they won't
   be after publish.

3. **Integration tests on CI** — question-integration-tests. Unit tests
   pass without tmux. Integration tests need a live tmux, which means
   CI needs tmux installed. Platform-specific behavior (session-010:
   5 fixes for Linux) means tests must run on both macOS and Linux.

4. **Documentation** — question-documentation. Rustdoc coverage, a
   README with usage examples, error handling guidance. The CLAUDE.md
   has the architecture but that's for AI agents, not Rust developers.

5. **Platform matrix** — question-platform-matrix. Which tmux versions?
   3.2+? 3.5+? Which OS? macOS, Linux. Which architectures? arm64, amd64.

6. **Async feature flag** — question-async-feature. Tokio consumers will
   want async. The sync-only model is a design choice (no tokio dep) but
   may limit adoption.

## The value proposition

"The only Rust crate for programmatic tmux control via the -CC protocol."
That's a strong position. Nobody else has built this. The question is
whether the maintenance burden of a public crate is worth the adoption.

## Tensions

- Public crate means semver commitments. Breaking the API after publish
  requires a major version bump. The API hasn't been designed for
  stability — it was designed for forestage's needs.
- More consumers means more platform edge cases. Session-010 showed
  that macOS and Linux tmux behave differently in ways unit tests
  don't catch.
- The sync-only model is deliberate (no tokio = smaller dependency tree)
  but async is expected in the Rust ecosystem. An async feature flag
  adds maintenance surface.
- Is the audience big enough? How many Rust projects need programmatic
  tmux control? It's a niche — but niches with no alternatives tend
  to be grateful.
