# CI red: 10_playable smoke fails post lock-dwell - guns on Space also fires FlightBurnInput, ship overruns the beacon

- STATUS: IN_PROGRESS
- PRIORITY: 90
- TAGS: v0.8.0,bug,examples,test

CI run 29658523978 (master, 2026-07-18): `harnessed_examples_reach_playing_without_panic`
fails on `10_playable` with the backstop panic
`playable: the run never finished (raised=true combat=true fired=true travel=true goto=false done=false)`
and scenario variables `target_down=1, leg=0, arrived=1, scenario_elapsed=6.43`.

## Diagnosis

Root cause is a latent input-binding collision in the example, exposed by the
new lock-acquisition dwell mechanic:

1. `examples/10_playable.rs` maps the "guns" section to `KeyCode::Space` (and
   `GamepadButton::RightTrigger`). Both of those are ALSO bound to
   `FlightBurnInput` in the default flight rig
   (`crates/nova_gameplay/src/input/player.rs`, bindings W / Space /
   RightTrigger). The script holds Space through the whole kill window, so the
   ship fires AND burns the main drive, then coasts Newtonian toward the
   prey/beacon.
2. Evidence in the failing log: the ship plows through the kill's mesh-fragment
   debris at z=-40 (the damage-0.00 impact spam) and physically overruns the
   beacon's 18u trigger area - `arrived=1.0` fired even though GOTO never
   engaged. The area OnEnter filter requires `other_id="player_ship"` and only
   the ship root carries that EntityId (bullets do not), so the ship really
   flew there.
3. As the ship passes the beacon (14u off-axis at spawn), the beacon's bearing
   swings outside the radar's 18-degree cone, so the post-kill travel sweep
   never resolves a candidate: `TravelLock` stays None, G is never pressed,
   backstop panics at WINDOW_SECS-0.5.

Why it passed before: the overrun always happened, but pre-dwell the travel
lock committed the INSTANT the sweep resolved the beacon - right after
lowering, while the beacon was still in-cone. The acquisition dwell
(edb48a4c, 2026-07-17, ~0.6s+ of steady candidate required) landed AFTER the
last green smoke run (Jul 16 19:55, run 29529924571); every CI run in between
died earlier in the pipeline (fmt gate, gauntlet version test, unit tests), so
the failing run was the FIRST time the smoke suite met the dwell mechanic.
Under llvmpipe the throttled sim (~6.4 virtual seconds inside the 18s wall
window; Bevy Time max-delta cap) makes the dwell race unwinnable once the ship
is coasting away.

## Steps

- [x] Map the example's "guns" to the shipped fire bindings
      (`MouseButton::Left`, `GamepadButton::RightTrigger2`) so firing no
      longer collides with FlightBurnInput; script fires with LMB.
- [x] Update the example's doc comments (header narrative + beat comments
      mention Space).
- [x] Bump WINDOW_SECS 18 -> 24: the dwell added ~1.25 virtual seconds of
      mandatory lock time since the window was tuned, and the throttled CI
      sim only fits ~0.37x wall.
- [x] Verify: fmt + check; run the example headless under Xvfb with
      BCS_AUTOPILOT=1 and confirm `autopilot: cycle complete, no panic`.

## Fix record (2026-07-19)

Landed on branch `fix/10-playable-smoke-guns-burn`. Verified headless
(Xvfb :87, BCS_AUTOPILOT=1): `nova harness: reached Playing`, then
`playable: prey destroyed, waypoint locked, GOTO closing at 0.12 u/s` ~3.2 s
after Playing, then `autopilot: cycle complete, no panic (t=24.0s)`, exit 0,
no "Encountered an error in command" lines. CI validation on the PR is the
remaining DoD item.

Alternatives considered:

- Keep Space and tap-fire in bursts: still burns intermittently, and drifts
  from the "exact gestures a player would make" charter (the shipped game
  fires on LMB).
- Only extend WINDOW_SECS: cannot work - once the ship coasts past the
  beacon the bearing leaves the 18-degree radar cone permanently, so no
  window is long enough.
- Counter-burn after the kill (script presses X/STOP): more script, and it
  keeps the misleading binding collision in the example that others copy.

Reflection: the failing assertion (goto=false) pointed at the travel sweep,
but the decisive clue was the SIDE variable `arrived=1.0` - impossible under
the intended choreography, therefore the ship had moved. Chasing the
"impossible" variable found the burn collision faster than staring at the
lock code did; also, the suspicious-looking new mechanics (lock_refire_secs,
ORBIT trim) were both innocent - reading their semantics before blaming them
avoided two wrong fixes. Note for authors: PlayerControllerConfig
input_mapping bindings silently overlay the flight rig's bindings
(consume_input: false), so any section mapped to W/S/Space/RightTrigger will
double-drive flight; a content lint for that overlap could catch this class
at author time.

## DoD

`BCS_AUTOPILOT=1 cargo run --example 10_playable --features debug` under a
virtual display reaches Playing, logs
`playable: prey destroyed, waypoint locked, GOTO closing at ...` and exits
cleanly; CI smoke suite green on the PR.
