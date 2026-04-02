# tmux-cmc Roadmap — Ideas

## Immediate (unblock aclaude distribution)

- [ ] `cargo publish --dry-run` passes clean
- [ ] Semver policy decided (0.1.x patch, 0.x.0 breaking until 1.0)
- [ ] Release CI: push tag → cargo publish via GitHub Actions
- [ ] aclaude Cargo.toml migrates from path → git tag or crates.io

→ Crystallizes into: question-crates-publish, question-aclaude-dep-source

## Short-term (quality and correctness)

- [ ] Integration test suite against live tmux runs in CI
- [ ] Notification coverage audit — which tmux notification types are missing?
- [ ] Concurrent send_command() stress test (multiple threads sharing one Client)
- [ ] Handle tmux subprocess death gracefully (Disconnected error propagation)

→ Crystallizes into: question-integration-tests, question-notification-coverage

## Medium-term (ecosystem)

- [ ] `async` feature flag — tokio wrapper over the sync API
  - Only worth doing once there is a concrete async consumer
  - Notifications → tokio::sync::broadcast
- [ ] Decide: does marvel need tmux-cmc or does Go keep its own subprocess driver?
  - Likely answer: Go keeps its own; Rust notification stream is not needed there

→ Crystallizes into: question-async-feature, question-marvel-tmux-driver

## Open questions that need a probe

See `_kos/nodes/frontier/` for formal question nodes.
