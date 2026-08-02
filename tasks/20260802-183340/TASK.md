# Port the harness completion protocol into nova_autopilot

- STATUS: OPEN
- PRIORITY: 98
- TAGS: v0.10.0, tooling, autopilot
- KIND: TASK
- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183336

## Story

Port the completion protocol into `nova_autopilot::completion`. Collectors
register at plugin build and report done; a watcher writes `AppExit::Success`
only when the pending set empties, and a deadline backstop error-exits naming
the laggards. Success negotiates; failures abort directly.

## Steps

- [ ] Write the five App-driven tests first in
      `crates/nova_autopilot/src/completion.rs` (`#[cfg(test)] mod tests`,
      `MinimalPlugins`, draining `Messages<AppExit>`): negotiated success,
      single-collector parity, deadline error naming the laggards, unknown
      `done`, duplicate registration. Confirm red.
- [ ] Port the protocol into the same file: the module doc (protocol rules,
      success-negotiates / failure-aborts), `HarnessCompletion` with
      `done`/`is_pending`/`others_pending`, `register(&mut App, &'static str)`,
      the `Last` `completion_watch` system, and the constants `AUTOPILOT`,
      `SCREENSHOT`, `DEADLINE_ENV`, `DEFAULT_DEADLINE_SECS`.
- [ ] Set `DEADLINE_ENV = "NOVA_AUTOPILOT_DEADLINE"`; drop the `BCS_` name with
      no alias. Keep `DEFAULT_DEADLINE_SECS = 120.0`.
- [ ] Every public item carries a doc comment - the crate is
      `#![warn(missing_docs)]` and the workspace denies warnings.
- [ ] Green the suite and the crate's rustdoc.

## Definition of Done

- Every collector must finish before the app exits successfully.
  (test: `exits_success_only_when_every_collector_is_done`)
- An expired deadline is an error exit that names the pending collectors.
  (test: `deadline_error_exits_naming_the_laggards`)
- The module suite is green with all five tests present, not vacuously green
  on an empty module.
  (cmd: `nix develop --command cargo test --lib -p nova_autopilot completion 2>&1 | rg -q 'test result: ok. 5 passed'`)
- The deadline env is Nova's, with no BCS alias left in the crate.
  (cmd: `rg -q 'NOVA_AUTOPILOT_DEADLINE' crates/nova_autopilot/src/completion.rs && ! rg -n 'BCS_' crates/nova_autopilot/src`)
- Rustdoc is clean.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Notes

- Parent: `20260802-120019`. Depends on the crate shell.
- `register` stays public: `nova_probe`'s frame-time capture registers its own
  `capture` collector through it.
- Source: `/home/alex/personal/bevy-common-systems/src/completion.rs`. Both
  crates are on `bevy 0.19.0`, so the `MessageWriter<AppExit>` /
  `Messages<AppExit>` API ports verbatim; no API adaptation expected.
- Type name `HarnessCompletion` is kept as-is. Renaming it is not needed by any
  DoD proof and no sibling task references it; revisit under the docs task
  `20260802-183355` if the crate's vocabulary should shed "harness".
- No `REEL` collector constant yet - the reel joins the protocol in
  `20260802-183349`, which adds its own name there.
- Prelude re-exports stay out of scope; `20260802-183355` owns the prelude.
- The `register` duplicate-`add_systems` NOTE comment is load-bearing (explains
  why re-adding `completion_watch` per registrant is safe); port it verbatim.
