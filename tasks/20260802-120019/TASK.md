# Move the automation harness into nova_autopilot

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.10.0,tooling,autopilot,crates
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955

## Story

Create `crates/nova_autopilot` and move Nova's automation driver out of
`bevy-common-systems`. Nova must own the state/input runway, loop and
self-completion behavior, shared collector completion, and settled screenshot
driver needed by its examples. Preserve behavior while renaming the internal
activation contract from `BCS_*` to `NOVA_*`.

## Steps

- [ ] Inventory every BCS harness type, completion API, env name, and direct or
      re-exported caller in `nova_debug`, `nova_probe`, examples, scripts, tests,
      and docs.
- [ ] Add the workspace crate with crate docs, public API docs, a prelude, and
      focused modules for the driver and completion protocol.
- [ ] Port the currently used behavior with App-driven tests: state runway,
      input timing after `InputSystems`, self-completion timeout, loop reset,
      collector negotiation, screenshot settling, and failure exit.
- [ ] Rewire `nova_debug`, `nova_probe`, and the full example fleet. Rename
      repository-owned env contracts and help text to `NOVA_AUTOPILOT`,
      `NOVA_SHOT`, and `NOVA_REEL`; do not retain BCS compatibility aliases.
- [ ] Remove the BCS `debug::harness` dependency surface from Nova while
      retaining unrelated BCS gameplay helpers.
- [ ] Update developer wiki, examples, scripts, and AGENTS command references.

## Definition of Done

- `nova_autopilot` owns the automation and completion APIs used by Nova.
  (test: `autopilot_completion_waits_for_every_collector`)
- The migrated example fleet uses no BCS harness path or activation variable.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|debug::harness" crates examples scripts --glob '*.rs' --glob '*.py' --glob '*.sh'`)
- The crate's real App-driven driver tests cover success, loop, timeout, input,
  and screenshot completion. (cmd: `nix develop --command cargo test --lib -p nova_autopilot`)
- Public items are exported through the crate prelude and rustdoc is clean.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Notes

- Source behavior: `/home/alex/personal/bevy-common-systems/src/debug/harness/`.
- Current Nova wrapper: `crates/nova_debug/src/harness.rs`.
- Direct extraction first. Nova-specific capability belongs to the next child.
- No changes land in the BCS checkout.
