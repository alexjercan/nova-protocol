# RCS mouse control: delta-driven instead of virtual-joystick accumulate (playtest feel)

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.7.0, feature, input, playtest

## Goal

Playtest 2026-07-18: the RCS held-direction virtual joystick is "way too hard to
control" - the accumulated offset keeps pushing after the mouse stops. Switch to
DELTA-driven: force follows the mouse's per-frame motion and stops when the mouse
stops (spike Q1, the runner-up model). Reverses the spike's held-direction call
based on how it actually felt. RCS is disabled in the mainline (verb withheld),
so this only changes RCS-granting ships / the rework.

## Steps

- [x] `on_rcs_aim` (crates/nova_gameplay/src/input/player.rs): SET
  `RcsIntent.x/.z` from the current mouse delta (`clamp(fire.value * sens, -1,
  1)`) instead of `accumulate_rcs_axis` - each frame's motion IS the command, not
  a running offset.
- [x] Add a player-only decay so the intent fades to zero when input stops:
  `decay_player_rcs_intent` in flight.rs (FixedUpdate, in the `NovaFlightSystems`
  chain AFTER `rcs_burn_system`), gated `With<RcsActive>` - `intent.0 *=
  RCS_PLAYER_INTENT_DECAY` (0.4), snapped to zero below an epsilon.
  RcsActive is the player's SHIFT modal, so the autopilot's own `RcsIntent`
  (task 20260718-122932) is untouched. Registered in the real plugin chain AND
  the flight test harness chain.
- [x] Scroll (Y) rides the same decay: each notch is a transient nudge that fades
  (no code change to the scroll observers - the decay makes their accumulate
  transient).
- [x] Tests: updated the player.rs mouse test to assert SET-not-accumulate (two
  separate motions -> intent reflects the LAST delta, not the sum); added a flight.rs
  test that a player (RcsActive) `RcsIntent` decays to ~zero over ticks with no
  fresh input while an autopilot (no RcsActive) intent does not. Re-ran the
  `flight::` + `input::player::tests::rcs` suites: 77 passed, 0 failed - the
  autopilot RCS tests still pass (they have no RcsActive so the decay skips them).

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Q1 was held-direction; this reverts to
delta after playtest). Parent family: 20260717-105406 (CLOSED). `on_rcs_aim` +
`RCS_AIM_SENSITIVITY` at player.rs ~1013/1071; `rcs_burn_system` + the flight
chain at flight.rs ~2137 / the `NovaFlightSystems` `.add_systems`. Sensitivity
may need a by-eye retune now that it maps per-frame delta -> force directly.

## Close-out (2026-07-18)

Delivered the delta-driven control model. Design record in NOTES.md.

- `on_rcs_aim` now SETS `RcsIntent.x/.z` from the current frame's mouse delta
  (`(fire.value * RCS_AIM_SENSITIVITY).clamp(-1, 1)`) instead of
  `accumulate_rcs_axis`. Each frame's motion is the whole command; stop moving
  the mouse and the next frame's write is zero.
- `decay_player_rcs_intent` (flight.rs) multiplies the player's `RcsIntent` by
  `RCS_PLAYER_INTENT_DECAY = 0.4` each FixedUpdate and snaps to zero below
  1e-4, gated `With<RcsActive>`. This makes the write transient: without it a
  single non-zero write would linger at cap forever (same runaway the playtest
  complained about); with it, the intent bleeds off within a few ticks once the
  mouse (or scroll) stops feeding it. Gating on `RcsActive` leaves the
  autopilot's own `RcsIntent` (which has no `RcsActive`) untouched, so the
  terminal-settle path from task 20260718-122932 is unchanged.
- Scroll-Y needed no code change: the same decay makes each notch's
  `accumulate_rcs_axis` write a transient nudge.
- Kept `RCS_AIM_SENSITIVITY = 0.02`; the delta mapping plus 0.4 decay felt
  controllable in the harness numbers. A live retune is cheap to do later if
  the feel needs it (single constant).

Alternatives considered: (a) a lower held-direction cap - rejected, it keeps the
joystick model the playtest disliked; (b) decaying inside `rcs_burn_system`
itself - rejected, it would couple the burn to the modal and also decay the
autopilot's intent. A standalone gated system keeps the two intent sources
independent.

Difficulty: the decay lives in the same FixedUpdate chain the autopilot RCS
loop runs in, so the `flight::` suite had to confirm no autopilot regression.
It passes because autopilot ships carry `RcsIntent` but not `RcsActive`, and
the new test pins exactly that split (player decays to ~0, auto stays > 0.5).