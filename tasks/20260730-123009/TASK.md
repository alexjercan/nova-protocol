# Investigate: combat lock lets go of locked enemies (intended decay or defect?)

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.9.0,bug,gameplay,targeting,feedback

## Story

Owner playtest 2026-07-30 (feedback wave):

> sometimes the ship loses "radar" focus on locked enemies, is it intended or a
> bug?

The question IS the deliverable: identify the mechanism that drops the lock,
with real numbers, and say plainly whether it is a shipped rule or a defect.

## Understanding (2026-07-30) - the candidate mechanisms

`crates/nova_gameplay/src/input/targeting.rs` clears a combat lock in five
places. Any of them can present as "it just let go":

1. **Idle decay (prime suspect).** `COMBAT_DECAY_SECS = 30.0`. While a combat
   lock exists and `WeaponsRaised` is false, `CombatDecay` accumulates
   `time.delta_secs()`; at 30 s the lock clears and the weapons safety
   re-engages. Raising the weapons resets the clock. Nothing on screen counts
   this down, so from the cockpit it reads as a random drop.
2. **Out of range.** `collect_lockable` re-gates every frame;
   `range_hysteresis` (1.15) widens the incumbent's gate, but a ship pulling
   away past `TARGETING_MAX_RANGE` (20000) still drops.
3. **Allegiance flip.** A hostile turning non-hostile clears the lock
   deliberately (a scripted surrender must not keep the guns hot).
4. **Death / despawn.**
5. **Staged tap-clear.** A CTRL tap the player did not mean as a tap - note
   `RADAR_TAP_SECS` is the threshold between "tap = clear" and "hold =
   search", so a short intended hold clears the lock instead. Worth measuring:
   an accidental clear here would be a real bug, not a rule.

6. **Firing does NOT reset the decay clock (new, 2026-07-30 - prime suspect,
   demotes #1).** `CombatDecay` is written in exactly one file
   (`input/targeting.rs`); a workspace-wide grep finds no other writer. Yet
   three doc comments in that file promise firing resets it - "(the raised
   stance; firing joins in 20260713-082337)" - and task `20260713-082337` is
   CLOSED without that wiring landing. Combined with
   `derive_control_mode_and_raised` (`camera_controller.rs:843`), which sets
   `WeaponsRaised` only while the combat button is HELD, and
   `WeaponsHot = raised OR combat lock`, a player can legitimately fight with
   the stance lowered: lock a hostile, do not hold the combat button, shoot at
   it for 30 s, and the lock drops MID-FIGHT. That fits "sometimes the ship
   loses radar focus on locked enemies" better than a purely idle decay does.
   Documented intent vs shipped behaviour disagree, so this is a defect, not a
   rule - pending the rig's confirmation.

Note the wording: "focus" may also mean `LockFocus`, the separate
`FOCUS_TIME = 1.5` s component-lock dwell, which resets whenever the lock
target changes. The investigation must distinguish "the LOCK dropped" from
"the FOCUS dwell reset" before concluding.

## Owner decision (2026-07-30)

If it turns out to be the 30 s idle decay: KEEP the rule, but make it visible -
the lock must be seen winding down rather than vanishing. So this task is an
investigation whose likely output is a small HUD change, not a targeting
change. Any other mechanism found is re-triaged on its own merits.

## Owner decisions (2026-07-30, plan gate)

Recorded in `DECISION.md`:

- **Fire-reset fix lands HERE**, not as a follow-up task, if the rig confirms
  mechanism 6.
- **The wind-down cue is the EXISTING combat reticle**
  (`TorpedoTargetReticleMarker`, an `ImageNode` in `hud/torpedo_target.rs`), not
  a new widget: a system drives its alpha from `CombatDecay` over the last
  seconds of the window - it dims and pulses faster as the lock lets go, then
  the existing unlatch ghost pops. No new marker, no new spawn.

## Deviation from the plan (2026-07-30, recorded during work)

The Steps said a TEST-ONLY observer of the clear branch would be enough, and
explicitly "do not ship a debug component". What shipped instead is a real
production message, `CombatLockDropped { target, reason, idle_secs }`, written
by the upkeep at each of its four drop branches. Reasons: a test-only rig would
have had to INFER the branch from leftover world state (exactly what the task
warned against - `OutOfRange` and `TargetGone` are indistinguishable after the
fact without re-running the gate), and the same message is the natural hook for
a future "LOCK LOST: OUT OF RANGE" cue. It is a message, not a debug component,
and carries no per-frame cost when nothing drops.

## Steps

- [x] Build the evidence rig first: an App-driven harness in
      `input/targeting.rs`'s test module that acquires a combat lock on a
      hostile and then runs the two contrasting scenarios - (a) truly idle,
      (b) FIRING with the stance lowered - recording the elapsed time and the
      branch that clears the lock in each. Extend the existing `locked_world`
      / `gesture_app` rigs rather than writing a bespoke one.
- [x] Instrument the clear paths so the rig reports WHICH one fired rather
      than inferring it (a test-only observer/log of the branch taken is
      enough - do not ship a debug component). Real numbers into NOTES.md.
- [x] Confirm or falsify mechanism 6 explicitly: assert whether a player fire
      event resets `CombatDecay`. This is the fail-first test for the fix.
- [x] Separately check the tap/hold boundary: measure how long a "short hold"
      has to be to be read as a hold, and whether a normal re-lock gesture can
      land under `RADAR_TAP_SECS` (0.25 s) and clear instead. Record the
      measurement either way.
- [x] Distinguish lock-drop from focus-dwell reset (`LockFocus`, `FOCUS_TIME`
      1.5 s) in the recorded evidence.
- [x] Write the verdict in NOTES.md: mechanism, numbers, intended-or-bug, per
      mechanism 1-6.
- [x] Fix mechanism 6 if confirmed: firing resets `CombatDecay` exactly as the
      raised stance does, closing the gap the shipped comments already
      promise. Keep `COMBAT_DECAY_SECS` unchanged. Update the three stale
      "firing joins in 20260713-082337" comments to state the landed
      behaviour.
- [x] Add the wind-down cue per the owner decision above, pinned by a HUD rig
      that asserts the reticle alpha at t < ramp start vs inside the ramp.
- [x] Pin the non-behaviour: regressions asserting a lock does NOT drop for
      the paths the investigation rules out (in particular: not from a normal
      re-lock gesture, and - after the fix - not while the player is firing).

## Definition of Done

1. The rig identifies, by name, the branch that clears the lock in the reported
   situation, with the elapsed time recorded (test: the evidence rig; numbers
   in NOTES.md).
2. The verdict - intended rule or defect - is written down with its evidence
   (manual: owner reads NOTES.md).
3. A regression pins the ruled-out paths so the answer cannot silently rot
   (test).
4. If the decay is confirmed: the lock is visibly winding down before it clears
   (test: a HUD rig over the cue; manual: owner sees it in flight).
5. If mechanism 6 is confirmed: firing resets the decay clock, proven by a test
   that failed first for the right reason (test), and no comment in the tree
   still promises unlanded fire-reset behaviour
   (cmd: `grep -rn "firing joins in" crates/` returns nothing).
6. A probe run of a combat example is OK/WARN with no new targeting warnings
   (cmd: `cargo run -p nova_probe -- run <combat example>`).

## Notes

A cycle that ends in a falsification is a legitimate outcome here: if the rig
cannot make the lock drop for any reason the owner would call a bug, the
deliverable is the evidence plus the pin.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED
