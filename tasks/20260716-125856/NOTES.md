# Scenario outcome frame - design record (task 20260716-125856)

## Shape

`Outcome` is presentation-only vocabulary: it declares Victory or Defeat (plus
an optional flavor line) and the overlay renders it. The CONSEQUENCE stays
composed from what already existed: a lingering `NextScenario` queued next to
the action becomes the Continue (Victory) or Retry (Defeat) button; nothing
queued means the story ends here and the only button is Main Menu, with Enter
exiting there too. Rejected alternative: an outcome action that also carries
"what happens next" (retry/next/menu enum) - it would duplicate NextScenario's
job, and every consumer (campaign mod chapters, gauntlet, the slice) would
need both surfaces kept in sync.

Data flow: the action `push_command`s into the event world's queue (the
HintEmphasis pattern, graceful `get_resource_mut` so headless script rigs
without the loader do not panic); the drained command writes
`CurrentOutcome` (a resource init'd by `ScenarioLoaderPlugin`); nova_menu's
`sync_outcome_overlay` mirrors that resource into UI, rebuilding on change
(outcomes flip at most once per scenario). Teardown clears the resource in
`teardown_scenario_entities` next to the emphasis clear - same reset class
(state-diff-aliases-reset) - so retries, chapter switches and menu exits all
start clean.

## The Enter key and the buttons are one mechanism

`NovaEventWorld::release_lingering_next()` is the single release path; the
loader's `on_next_input` (Enter/DPadDown) and the overlay's primary button
both call it. The advance decision itself is a pure table
(`decide_advance`): paused ignores, a queued switch wins, an outcome with
nothing queued exits to MainMenu, bare Enter stays inert. User direction
2026-07-16: the old press-Enter-only flow was "scuffed" - the buttons are the
fix; the key stays for keyboard/gamepad parity (DPadDown was already bound).

## Cursor

The overlay needs a pointer, so declaring an outcome frees the cursor
(`sync_outcome_cursor`). Re-grab CANNOT ride the outcome clearing: on Defeat
the player ship is already despawned, and a Retry reloads the scenario
WITHOUT a state transition, so the editor's OnEnter(Scenario) grab never
re-fires. Instead `regrab_cursor_on_player_spawn` (an `On<Add,
PlayerSpaceshipMarker>` observer) grabs when the NEXT ship spawns, guarded to
Playing + unpaused (and never in debug builds, mirroring the existing grabs).
The observer uses a plain Query, not `Single` - Single's skip-when-unsatisfied
is a system guarantee I did not verify for observers, and headless rigs have
no window.

## Deliberate v1 scope cuts

- No clock pause while the overlay is up: the pause menu is the app's single
  clock-pauser (review R1.6 of the pause task warned the unpause is an
  unconditional stomp), and a live world behind the banner - debris drifting -
  is the better read anyway. Consequence: on Victory the surviving ship still
  accepts flight/weapon input behind the modal; harmless today (the enemy is
  dead, picks are blocked by the overlay), revisit if a playtest complains.
- No HUD chrome suppression and no victory/defeat audio stinger (no such
  asset exists; the overlay's buttons DO click - the global MenuButton
  observer covers them for free).
- The banner/panel is the pause overlay's visual family (nova_ui theme,
  gold/red semantic accents), sized 320px; real typography lands with the
  fonts task (20260714-214329).

## Evidence rig (render-output-eyeball)

`tasks/20260716-125856/outcome_probe.png` - 1920x1080 Xvfb capture of the
real app showing the Defeat overlay over the loaded shakedown scene: red
DEFEAT banner, message, Retry + Main Menu buttons, key hint, HUD beneath the
dim layer. Rig: a throwaway `examples/99_outcome_probe.rs` (not committed;
this file documents it) that boots `editor_app(true)` + `nova_autopilot()` +
`nova_screenshot()`, clicks "New Game Button" via `world.trigger(Activate)`,
then in Playing sets `NovaEventWorld.next_scenario = Some(shakedown_run,
linger: true)` and `CurrentOutcome.0 = Some(Defeat, "Your ship broke apart in
the belt.")` directly (standing in for the 12-beat death path), waits 30
frames, spawns `Screenshot::primary_window()` + `save_to_disk`. Command:
`Xvfb :99 & DISPLAY=:99 BCS_AUTOPILOT=1 cargo run --example 99_outcome_probe
--features debug`. The full production chain (real death -> OnDestroyed ->
action -> overlay) gets its CI pin in the slice's example 19
(task 20260708-203659).

## Difficulties

- `cargo test -p nova_scenario` alone does not compile: the crate's serde
  round-trip tests rely on workspace feature unification (nova_assets ->
  nova_modding -> nova_scenario/serde). Pre-existing; run scenario tests
  together with a serde-enabling sibling (`-p nova_scenario -p nova_menu`) or
  workspace-wide, as CI does.
- The probe example initially failed to compile: root-crate examples cannot
  `use nova_scenario::...` directly; go through the re-export
  (`nova_protocol::nova_scenario::prelude::*`).

## Review round 1 addendum (R1.1 fix)

The out-of-context review caught a real MAJOR the Defeat-only probe masked:
pause/unpause over a live VICTORY overlay re-locked the cursor
(restore_cursor saw Playing + a live ship and knew nothing about outcomes),
stranding the buttons in release builds. Fix: restore_cursor and
regrab_cursor_on_player_spawn skip when an outcome is declared; regression
test `pause_cycle_over_a_live_outcome_keeps_the_cursor_free` (with a
cfg(not(debug)) delivery guard proving the cycle DOES re-grab once the
outcome clears - debug builds never grab, so the guard half only runs on
the release-features suite). Also from the round: overlay rebuilds when a
switch is queued later than the outcome (R1.3, snapshot on the marker),
instant-switch swallowing is documented + traced (R1.2), and the
outcome-below-pause z relation is pinned (R1.7).
