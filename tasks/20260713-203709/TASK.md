# Harnessed pin gap: remove/despawn warns bypass the fallback-to-panic error handler

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.5.2,testing,harness

## Goal

The 20260713-175352 pin (examples/13_menu_newgame.rs) swaps
`FallbackErrorHandler` to panic and claims "any command error on these
transitions now fails CI". Discovered during 20260712-115902: that is only
true for UNHANDLED commands (e.g. `insert`). `EntityCommands::remove` and
`despawn` bake in the WARN handler at queue time (bevy_ecs
commands/mod.rs: `queue_handled(_, warn)`), so their "Entity despawned"
errors log a warn and sail past the pin - the exact class of the
2026-07-12 playtest warn would NOT have failed the smoke test.

## Steps

- [ ] Extend tests/examples_smoke.rs to also FAIL a harnessed example whose
      stderr contains "Encountered an error in command" (the stable prefix
      of both the warn- and fallback-handled messages), so remove/despawn
      warns gate CI like panics do.
- [ ] Prove it bites (would-it-fail-without-it): a deliberate stale
      `remove` sabotage in one example must fail the smoke test; record the
      run, then revert the sabotage (commit first).
- [ ] Update examples/13_menu_newgame.rs's module doc (and the
      20260713-175352 pin-claim language it inherited) to state the real
      coverage: panics + unhandled command errors via the handler swap,
      handled warns via the stderr grep.

## Notes

- Discovered during task 20260712-115902 (teardown race sweep); the
  mechanism note lives in nova_gameplay's `test_log` module doc and that
  task's NOTES.md.
- Runs naturally after the examples rework (20260712-211352), which owns
  the smoke-test file and documents the pin convention this extends.
