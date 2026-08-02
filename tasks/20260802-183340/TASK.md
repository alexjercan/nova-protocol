# Port the harness completion protocol into nova_autopilot

- STATUS: OPEN
- PRIORITY: 98
- TAGS: v0.10.0,tooling,autopilot
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183336

## Story

Port the completion protocol into `nova_autopilot::completion`. Collectors
register at plugin build and report done; a watcher writes `AppExit::Success`
only when the pending set empties, and a deadline backstop error-exits naming
the laggards. Success negotiates; failures abort directly.

## Steps

- [ ] Port `HarnessCompletion`, `register`, the `Last` watcher, and the
      collector-name constants, renaming the deadline env to
      `NOVA_AUTOPILOT_DEADLINE`.
- [ ] Port the App-driven tests: negotiated success, single-collector parity,
      deadline error naming laggards, unknown `done`, duplicate registration.

## Definition of Done

- Every collector must finish before the app exits successfully.
  (test: `exits_success_only_when_every_collector_is_done`)
- An expired deadline is an error exit that names the pending collectors.
  (test: `deadline_error_exits_naming_the_laggards`)
- The module suite is green.
  (cmd: `nix develop --command cargo test --lib -p nova_autopilot completion`)

## Notes

- Parent: `20260802-120019`. Depends on the crate shell.
- `register` stays public: `nova_probe`'s frame-time capture registers its own
  `capture` collector through it.
- Source: `/home/alex/personal/bevy-common-systems/src/completion.rs`.
