# RCS control model: delta-driven, not virtual-joystick

## What changed

RCS mouse aim was a *held-direction virtual joystick*: `on_rcs_aim`
accumulated the mouse delta into `RcsIntent`, so a held offset kept commanding
a burn even after the mouse stopped moving. Playtest 2026-07-18 called this
"way too hard to control" - the ship kept translating toward the accumulated
offset until you actively pushed the mouse the other way.

It is now *delta-driven*: the force follows the mouse's per-frame motion and
fades when the mouse stops.

## Mechanism

Two coupled pieces:

1. `on_rcs_aim` (input/player.rs) SETS the intent from the current frame's
   delta instead of accumulating:

   ```rust
   let delta = (fire.value * RCS_AIM_SENSITIVITY).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
   intent.x = delta.x;
   intent.z = delta.y;
   ```

   A frame with no mouse motion writes zero. The command is the instantaneous
   motion, not a running position.

2. `decay_player_rcs_intent` (flight.rs, FixedUpdate, gated `With<RcsActive>`)
   bleeds the intent toward zero each tick:

   ```rust
   intent.0 *= RCS_PLAYER_INTENT_DECAY; // 0.4
   if intent.0.length_squared() < 1e-4 { intent.0 = Vec3::ZERO; }
   ```

   The observer only *writes* on input frames (via bevy_enhanced_input's
   `Fire` events); on frames with no input it does not run at all, so without
   a decay the last non-zero write would persist and keep burning at the cap.
   The decay makes every write transient: stop feeding input and the intent is
   effectively zero within ~3-4 fixed ticks.

Scroll-Y (the vertical nudge) needed no change: its `accumulate_rcs_axis`
write is now equally transient because the same decay bleeds it off.

## Why gate on `RcsActive`

`RcsActive` is the player's SHIFT modal marker. The autopilot writes its own
`RcsIntent` for terminal settle-to-rest (task 20260718-122932) on ships that
do NOT carry `RcsActive`. Gating the decay on `With<RcsActive>` means:

- player RCS: intent is transient (decays), matching the delta feel;
- autopilot RCS: intent is authoritative each tick (the autopilot rewrites it
  every loop from the position error), so it must NOT be decayed underneath.

The test `player_rcs_intent_decays_when_input_stops_but_autopilot_intent_does_not`
pins exactly this split.

## Tuning

- `RCS_AIM_SENSITIVITY = 0.02` (unchanged): per-frame mouse-delta pixels ->
  clamped [-1, 1] intent.
- `RCS_PLAYER_INTENT_DECAY = 0.4`: lower = snappier stop, higher = more glide.
  0.4 gives a ~3-4 tick fade. Both are single constants; retune by feel in a
  live playtest if needed.

## Relationship to the spike

The spike (tasks/20260718-122508/SPIKE.md, Q1) chose held-direction as the
primary and delta as the runner-up. This task reverses that call based on how
held-direction actually felt in the hand. The spike doc's reasoning still
stands as the record of why held-direction *looked* better on paper (a steady
offset is easier to reason about); the playtest is the evidence that the
runner-up wins in practice.
