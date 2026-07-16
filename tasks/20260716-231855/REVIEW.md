# Review: Gate the OnUpdate scenario pulse (fire_on_update) on Unpaused

- TASK: 20260716-231855
- BRANCH: gate-fire-on-update-pause

## Round 1

- VERDICT: APPROVE

Scope reviewed: `git diff master...gate-fire-on-update-pause` - the
`fire_on_update` run condition, a new App-driven test, two dev-wiki event
tables, and a CHANGELOG Fixes line.

Independently re-verified load-bearing claims (shared-session blind spot):

- **Delivery guard holds.** `on_update_pulse_freezes_while_paused_and_resumes_on_unpause`
  (loader.rs:1395+) never pauses the virtual clock - it only flips
  `PauseStates` - so it isolates the *state* gate. Delete
  `.and_then(in_state(PauseStates::Unpaused))` and the Paused phase keeps
  incrementing `count`, failing `assert_eq!(count, at_pause)`. The test would
  fail with the fix removed, and its Unpaused/resume phases prove the pulse
  still fires otherwise (not a "nothing happens" no-op assertion).
- **No new coupling.** `on_next_input` already reads a *panicking*
  `Res<State<PauseStates>>` (loader.rs:813), so any app running scenarios
  already hard-requires `PauseStates`. The `in_state` gate is strictly safer
  (false-if-missing, not a panic), so it introduces no new plugin dependency.
- **Single production registration.** Only loader.rs:266 gates in production;
  loader.rs:1348 is the pre-existing liveness test, correctly left ungated.
- **Siblings correctly left ungated (walked, not blanket).**
  `track_orbit_holds` / `track_player_locks` fire only on a `Time<Virtual>`
  delta threshold, and both pause paths freeze that clock (delta 0), and their
  source state (autopilot, player locks) is itself Unpaused-gated - so nothing
  newly fires. `apply_pending_skybox_swaps` is cosmetic and asset-load driven;
  finishing a queued swap under pause is harmless. The per-system rationale is
  captured in the code comment at loader.rs:248-263.
- **Both pause paths covered.** The ESC menu (nova_menu) and the outcome frame
  (nova_gameplay/plugin.rs:181) both set `PauseStates::Paused`, so the single
  `in_state(Unpaused)` gate covers both, matching the task Goal.

Checks: `cargo test -p nova_scenario --lib --features serde on_update` green
(2 passed); `cargo fmt -p nova_scenario --check` clean; the earlier
`.and(...)` deprecation warning was resolved by switching to `.and_then(...)`.
Full suite deferred to CI per the repo's local-test policy.

Docs: the two dev-wiki `OnUpdate` "every frame while live" rows now carry the
unpaused qualifier, and a CHANGELOG Fixes entry lands the player/modder-facing
note. No other surface documents the pulse timing.

No BLOCKER/MAJOR/MINOR/NIT findings. Clean diff, approved.
