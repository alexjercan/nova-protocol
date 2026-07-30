# Investigate: combat lock lets go of locked enemies (intended decay or defect?)

- STATUS: OPEN
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

Note the wording: "focus" may also mean `LockFocus`, the separate
`FOCUS_TIME = 1.5` s component-lock dwell, which resets whenever the lock
target changes. The investigation must distinguish "the LOCK dropped" from
"the FOCUS dwell reset" before concluding.

## Owner decision (2026-07-30)

If it turns out to be the 30 s idle decay: KEEP the rule, but make it visible -
the lock must be seen winding down rather than vanishing. So this task is an
investigation whose likely output is a small HUD change, not a targeting
change. Any other mechanism found is re-triaged on its own merits.

## Steps

- [ ] Build the evidence rig first: a harness run that acquires a combat lock
      on a hostile, then holds weapons LOWERED and does nothing, and records
      the exact frame/time the lock clears plus which branch cleared it.
      Prefer extending an existing targeting rig over a bespoke one.
- [ ] Instrument the five clear paths so the rig reports WHICH one fired,
      rather than inferring it. Real numbers into NOTES.md.
- [ ] Separately check the tap/hold boundary: measure how long a "short hold"
      has to be to be read as a hold, and whether a normal re-lock gesture can
      land under `RADAR_TAP_SECS` and clear instead. Record the measurement
      either way.
- [ ] Distinguish lock-drop from focus-dwell reset in the recorded evidence.
- [ ] Write the verdict in NOTES.md: mechanism, numbers, intended-or-bug.
- [ ] If the verdict is the idle decay: add the wind-down cue - the lock
      crosshair visibly decays over the last seconds of the window - and pin it
      with a rig. Keep `COMBAT_DECAY_SECS` unchanged.
- [ ] If the verdict is a real defect (accidental tap-clear, a hysteresis hole,
      a spurious allegiance flip): fix it here if it is small, otherwise file it
      as its own task with the evidence attached and close this one with the
      diagnosis.
- [ ] Pin the non-behaviour either way: a regression asserting a lock does NOT
      drop for the reasons the investigation ruled out.

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
5. A probe run of a combat example is OK/WARN with no new targeting warnings
   (cmd: `cargo run -p nova_probe -- run <combat example>`).

## Notes

A cycle that ends in a falsification is a legitimate outcome here: if the rig
cannot make the lock drop for any reason the owner would call a bug, the
deliverable is the evidence plus the pin.

## Flow State

- FLOW STEP: PLANNED
