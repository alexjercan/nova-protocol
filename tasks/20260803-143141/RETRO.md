# Retro: Fix the hud_range example smoke: the scripted run never reaches its last beat

- TASK: 20260803-143141
- BRANCH: fix/hud-range-runway
- REVIEW ROUNDS: 1

## Reproduction method (Step 1)

All runs: `Xvfb :99 -screen 0 1280x720x24`, then
`DISPLAY=:99 NOVA_AUTOPILOT=1 nix develop --command cargo run --example <ex> --features debug`.

The defect is load-dependent by construction, so the local lever is the
window/backstop arithmetic rather than the load itself. Only the DIFFERENCE
between the autopilot window and `load + 4.8 s` matters, so shrinking the
window is arithmetically the same as lengthening the load.

**A dead end worth recording.** Stalling the `Loading` state with a per-frame
`std::thread::sleep` (60 ms, then 110 ms) does NOT reproduce anything. Asset
loading runs on background threads against wall time, so sleeping makes each
loading frame longer without adding loading SECONDS to `Time`; the run reached
`Playing` with roughly the same elapsed and passed both times. Do not reach for
a sleep to fake a slow load in this harness.

### Red A - the prescribed lever, and the surprise

`hold(GameStates::Loading, 8.0)` -> `5.5`:

```
INFO nova_autopilot::autopilot: autopilot: cycle complete, no panic (t=5.5s)
INFO nova_autopilot::completion: harness completion: all collectors done, exiting
EXIT=0
```

Exit **0**. The last beat logged was `hud range: component highlight OK`
(`t > 3.5`); the kill beat (`t > 4.4`) and the final drop-assertion beat
(`t > 4.8`) never ran. This is a VACUOUS PASS, not the reported panic: the
backstop's `elapsed > 7.5` sits ABOVE the shortened window, so it never gets to
fire. The plan predicted the panic here; the truth is worse than the plan, and
it means the smoke suite could have gone green with the example's whole point
(indicators hide when their anchor dies) unexecuted.

### Red B - the reported CI panic, verbatim

`hold` back to `8.0`, backstop threshold `7.5` -> `5.0` (the same arithmetic
from the other side - the backstop now lands between the load and `load + 4.8`):

```
thread 'main' (3159191) panicked at examples/ui/hud_range.rs:340:9:
hud range: the scripted run never finished (ring=true lock=true goto=false drop=false)
EXIT=101
```

Matches the CI symptom in the Story. Both temporary edits were reverted
(`git checkout examples/ui/hud_range.rs`) and the tree confirmed clean before
implementing.

## Falsification transcript (Step 8, the manual DoD proof)

Applied AFTER the fix was committed (`a0e1e24b`), so the restore is a
`git checkout` back to the fix, never to the branch base.

### Falsify A - hud_range final beat gated off (`t > 4.8` -> `t > 999.0`)

```
ERROR nova_autopilot::autopilot: autopilot: timeline expired but the self-completing script never reported done (t=30.0s)

thread 'Compute Task Pool (11)' (3176464) panicked at examples/ui/hud_range.rs:110:9:
hud range: run ended with the scripted run unfinished (ring=true lock=true goto=true drop=false)
EXIT=101
```

### Falsify B - com_range assert beat gated off (`t > 4.3` -> `t > 999.0`)

```
ERROR nova_autopilot::autopilot: autopilot: timeline expired but the self-completing script never reported done (t=30.0s)

thread 'Compute Task Pool (1)' (3178933) panicked at examples/sections/com_range.rs:97:9:
com range: run ended with the scripted run unfinished (spun=true kills=2)
EXIT=101
```

Both loud paths fire, in sequence: the harness writes `AppExit::error` naming
the expired runway, and the in-example guard then panics naming exactly how far
the walk got. A stalled script cannot exit 0 through either. Both sabotages
reverted; tree clean.

## What went well

- The falsification was worth more than the fix. It proved the two independent
  loud paths compose rather than mask each other, which is the property the old
  single backstop only claimed to have.
- Reproducing before touching anything caught that the plan's stated symptom
  was the LESS severe of the defect's two faces. A fix written to the plan's
  description alone would still have been correct, but the record would have
  understated the risk.

## What went wrong

- The plan asserted `hold` -> 5.5 would produce the panic. It produces a silent
  pass. The arithmetic was reasoned about from the log excerpts rather than run,
  and the backstop's position relative to the window got inverted.
- Two runs (~7 min) were spent on the `thread::sleep` load-stall idea before
  noticing that virtual-time accounting makes it a no-op for this purpose.

## Review round 1

APPROVE on the first round, no BLOCKER or MAJOR: one MINOR and three NITs, all
comment/doc accuracy. Diagnosis of the three questions the records answer:

- Breadth. ~120 lines across two byte-identical example scripts. Not a missed
  split: `com_range` carries the same defect latent, and converting one without
  the other would have left the next loaded runner to find it. The four
  wide-margin examples were deliberately excluded and recorded, not forgotten.
- Churn. Zero rework, but the one MINOR is a plan-level miss worth naming. Step
  6 scoped the doc sweep to the examples' *module-header* smoke docs, so the
  in-body comment at `hud_range.rs:1010` that justifies the kill-cam assert with
  "the 6s autopilot window ends before the linger does" survived a change that
  deleted the window. The question that would have caught it is not the
  from-scratch challenge - it is a plan-time grep of the concept being deleted
  ("window", "6s", the old `hold`) rather than a named file region. When a task
  removes a CONCEPT, the doc step should name the concept, not the location.
- Context. The implementation and the review ran in separate contexts by
  design; round 1 went to an out-of-context reviewer that re-ran every `cmd:`
  proof and both live Xvfb runs independently. No compaction or threshold
  crossing was observed. The flow itself crossed a context cut between WORKING
  and REVIEWING, and disk state alone was sufficient to resume - the sprout
  worktree, not the main checkout, held the authoritative TASK.md.

The load-bearing claim the recording pass re-derived rather than accepted: the
in-example guard cannot catch a *silent success*, because `completion_watch` is
the sole `AppExit::Success` writer and only writes it with the pending set
empty. The guard is a diagnostic on an already-failing exit. That is what the
code and the records actually claim, so it stayed an observation - but it is the
kind of claim a summary would have carried through unexamined.

## What to improve next time

- When a change deletes a concept, sweep the concept by grep, not the file
  region the plan named. Step 6 said "module-header smoke docs" and the stale
  "6s autopilot window" comment 900 lines below survived into review.
- A guard keyed off a DIFFERENT clock than the thing it guards is not a guard,
  it is a second failure mode. When auditing a backstop, always ask which clock
  each side is on and whether the guard's threshold can fall outside the run.
- When a plan predicts a specific failure signature, run it before trusting the
  step ordering that depends on it; a "reproduce first" step that reproduces
  something else is a finding, not a nuisance.

## Action items

- The four remaining fixed-window examples (`screenshot_juice`,
  `screenshot_orbit`, `screenshot_combat`, `playable`) carry the same shape with
  wider margins - audit table in `NOTES.md`. Deliberately left; convert them if
  the pattern bites again.
- Review round 1 left one MINOR and three NITs unfixed and unblocking (stale
  kill-cam comment, constant `drop=` field in the guard panic, `+4s/+4.5s`
  header times vs the `4.4/4.8` beats, hardcoded `"NOVA_AUTOPILOT"` instead of
  `harness::AUTOPILOT_ENV` in both examples). Comment/doc accuracy only; fold
  them into whichever task next touches these two examples.

## Landing message

```
fix(examples): make hud_range and com_range script-owned runs

The hud_range smoke failed on loaded CI runners because two clocks
disagreed: the script timeline is relative to entering Playing, while the
backstop fired off the autopilot-window clock, so load cost ate the slack
and the window closed before the final beat ran.

Both examples now self-end. The 8s hold becomes a 30s runway, the script
reports done through HarnessCompletion on its last beat, and the
elapsed-based backstop is deleted. A run exits when the walk finishes,
however long the load took. Three loud paths survive: runway expiry with
the script pending is a harness error exit, any premature AppExit trips an
in-example guard naming the unfired beats, and a failed beat assertion
still panics. Verified by deliberately gating each final beat off.
```
