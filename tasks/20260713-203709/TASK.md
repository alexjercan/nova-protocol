# Harnessed pin gap: remove/despawn warns bypass the fallback-to-panic error handler

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.5.2,testing,harness

## Goal

The 20260713-175352 pin (examples/12_menu_newgame.rs, nee 13_menu_newgame) swaps
`FallbackErrorHandler` to panic and claims "any command error on these
transitions now fails CI". Discovered during 20260712-115902: that is only
true for UNHANDLED commands (e.g. `insert`). `EntityCommands::remove` and
`despawn` bake in the WARN handler at queue time (bevy_ecs
commands/mod.rs: `queue_handled(_, warn)`), so their "Entity despawned"
errors log a warn and sail past the pin - the exact class of the
2026-07-12 playtest warn would NOT have failed the smoke test.

## Steps

- [x] Extend tests/examples_smoke.rs to also FAIL a harnessed example whose
      stderr contains "Encountered an error in command" (the stable prefix
      of both the warn- and fallback-handled messages), so remove/despawn
      warns gate CI like panics do.
- [x] Prove it bites (would-it-fail-without-it): sabotage applied AFTER
      committing the gate - a deliberate stale `remove::<Transform>` on a
      despawned entity in 01_controller_section failed the suite in 10 s
      with "example 01_controller_section logged a command error"; reverted
      via git checkout, clean suite green after.
- [x] Update examples/12_menu_newgame.rs's module doc (and the
      20260713-175352 pin-claim language it inherited) to state the real
      coverage: panics + unhandled command errors via the handler swap,
      handled warns via the stderr grep.

## Notes

- Discovered during task 20260712-115902 (teardown race sweep); the
  mechanism note lives in nova_gameplay's `test_log` module doc and that
  task's NOTES.md.
- Runs naturally after the examples rework (20260712-211352), which owns
  the smoke-test file and documents the pin convention this extends.


## Record (2026-07-13)

What changed: one assertion in tests/examples_smoke.rs (stderr must not
contain "Encountered an error in command"), the module doc explaining the
two error-handling flavors, and the corrected pin claim in
12_menu_newgame. Proven by sabotage A/B (red in 10 s, green after revert;
full 12-example suite green with the gate armed). With this, all three
detection layers gate CI: panics, unhandled command errors (handler swap),
and handled command warns (this grep).

Self-reflection: nothing notable - a small, well-scoped task whose
mechanism was fully understood when it was filed (by 20260712-115902's
bevy-source dig); it went plan-to-green in one pass.
