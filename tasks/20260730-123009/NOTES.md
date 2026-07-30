# Evidence: why the combat lock lets go

Task 20260730-123009. Rig: `crates/nova_gameplay/src/input/targeting.rs`
tests (`the_evidence_rig_names_every_branch_that_drops_the_combat_lock`,
`a_held_trigger_resets_the_decay_so_the_lock_survives_a_long_fight`,
`the_tap_hold_boundary_sits_exactly_at_the_shared_threshold`,
`a_focus_dwell_reset_is_not_a_lock_drop`).

The rig does not INFER the cause. `update_contacts_and_locks` now names the
branch it took at each drop - a `debug!` line plus a
`CombatLockDropped { target, reason, idle_secs }` message - so the mechanism
below is read off the run instead of reconstructed from the leftover state.

## Verdict

**Both a rule and a defect, and the defect is what the owner saw.**

The 30 s idle decay is an intended, user-tuned rule (decision D4). It was
INVISIBLE, which is the owner's "it just let go" - fixed by making the reticle
wind down. But the rule's definition of "combat activity" was incomplete:
only the RAISED stance reset the clock, never firing. Since the stance is only
raised while the combat button is HELD, and `WeaponsHot = raised OR locked`, a
player can legitimately fight with it lowered - and did lose the lock 30 s into
a live fight. That half is a defect and is fixed here.

## Mechanism by mechanism

| # | Mechanism | Reason reported | Verdict |
|---|---|---|---|
| 1 | Idle decay | `IdleDecay`, `idle_secs = 30.0` | INTENDED rule; was invisible -> now visible |
| 2 | Out of range | `OutOfRange` at > 23000 u (20000 x 1.15 hysteresis) | INTENDED |
| 3 | Allegiance flip | `AllegianceFlip` | INTENDED (scripted surrender) |
| 4 | Death / despawn | `TargetGone` | INTENDED |
| 5 | Staged tap-clear | (its own path, `LockClearedToast`) | INTENDED, boundary is exact - see below |
| 6 | **Firing does not reset the decay** | `IdleDecay` mid-fight | **DEFECT - fixed** |

## Mechanism 6, measured (the fail-first red)

With the fix disabled, a player firing continuously at a locked, in-range,
still-hostile target, stance lowered:

```
step 29 at 690 m dropped the lock:
  [CombatLockDropped { target: 5v0, reason: IdleDecay, idle_secs: 30.0 }]

a held trigger resets the idle clock every frame
  left: 15.0   right: 0.0
```

The clock ran to 30.0 s while the trigger was down, and the drop named itself
`IdleDecay` - not out-of-range, not a flip, not a despawn. Confirmed
mechanism 6.

Root cause: `CombatDecay` had exactly ONE writer in the whole workspace
(`input/targeting.rs`), yet three doc comments there promised firing reset it
"once 20260713-082337 lands". That task closed without the wiring. The
comments described behaviour the tree never had; they are corrected now.

Fix: combat activity is the raised stance OR a held trigger on one of the
player's OWN weapon sections (`TurretSectionInput` / `TorpedoSectionInput`
under `ChildOf(ship)`) - the same shape `sense_hud_situations` already uses
for the "firing" HUD situation. `COMBAT_DECAY_SECS` is unchanged at 30.0.

## Mechanism 5, measured (tap/hold boundary)

`RADAR_TAP_SECS = 0.25` is ONE constant feeding both the `Hold` (search) and
`Tap` (clear) conditions, so there is no gap between them. Swept at 50 ms per
frame, 1..=8 frames held, with a lock already held and the stance raised:

| Hold | 50 | 100 | 150 | 200 | 250 | 300 | 350 | 400 ms |
|---|---|---|---|---|---|---|---|---|
| Outcome | clear | clear | clear | clear | commit | commit | commit | commit |

The flip is exactly at 250 ms, in both directions, with no frame that does
neither or both. A hold intended as a hold has to be under a quarter second to
read as a tap. NOT the reported mechanism - but the measurement is pinned so a
future threshold change cannot silently open a gap.

## Lock drop vs focus dwell

"Focus" is ambiguous in the report. `LockFocus` (the 1.5 s `FOCUS_TIME`
component-lock dwell) RESETS whenever the lock target changes, while the
combat lock itself stays latched and no drop is reported. Pinned by
`a_focus_dwell_reset_is_not_a_lock_drop`, so the two can never be confused in
a future investigation.

## The wind-down cue

`hud/torpedo_target.rs`: over the last `DECAY_WIND_DOWN_SECS` (5 s) of the
window the combat reticle's alpha falls toward `DECAY_MIN_ALPHA` (0.25) under
a pulse that quickens from 1.5 Hz to 6 Hz - 18 pulses across the window,
measured identically at 60, 144 and 600 fps. Continuous at the boundary (zero
progress = full alpha, no pulse depth), so the cue fades in rather than
stepping. Any combat activity zeroes the clock and the reticle is solid again
the same frame. No new widget - the decision to drive the existing reticle
rather than add a countdown arc is in DECISION.md.

The cue is a function of the DECAY CLOCK alone. The first cut multiplied the
RENDER clock by the swept frequency, which makes the instantaneous rate
`hz + elapsed * d(hz)/dt` and so grows with session uptime: simulated at
60 fps it gave 29 pulses over the window at t=0, 10 at t=60 s (a smooth slide,
no visible pulse) and 118 at t=300 s (frame-rate aliasing, a flicker). Caught
by out-of-context review (R1.1) and fixed by integrating the linear chirp;
`the_wind_down_pulse_is_the_same_at_any_uptime_or_frame_rate` fails with the
old formula restored (148 pulses at 60 fps) and passes with the fix.

## Probe run (DoD 5)

`cargo run -p nova_probe -- run broadside` (the combat example), 2026-07-30,
on the FINAL tree (commit `4ef68731`, after the review round):

```
probe: OK - probe-runs/4ef68731/broadside/report.html
  process_exit           PASS     1 pass(es), all clean exits
  run_completed          PASS     run_end at frame 188
  reached_playing        PASS     Playing at frame 53
  invariants_held        PASS     0 violations over 188 checked frames
  fps_within_baseline    SKIPPED  missing capture or baseline
  log_clean              PASS     0 panic/ERROR lines
  broadside              OK       measured 5/6  102s
```

Verdict OK. `fps_within_baseline` is SKIPPED = NOT MEASURED (no capture pass
and no baseline run), not "held". The warn lines in the run log are the
pre-existing ones; none mention lock, target, decay or reticle.

The first probe (commit `69dc505c`, same six statuses, run_end at frame 189)
was taken mid-cycle and so covered a superseded commit - the reviewer caught
that, and this run replaces it rather than shipping evidence from a tree that
no longer exists.

## What is pinned

- `a_live_in_range_hostile_lock_never_drops_for_any_other_reason` - 600
  simulated seconds of firing at a target drifting 400 -> 6400 m: zero drops,
  from any branch.
- `another_ships_trigger_never_holds_the_players_lock_open` - the activity
  test is scoped to the player's own sections.
- `releasing_the_trigger_resumes_the_decay_from_zero` - the fix widens what
  counts as combat, it does not disable the rule.
