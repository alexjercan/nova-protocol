# Retro: Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- TASK: 20260804-093950
- BRANCH: feat/sections-multi-round-invariants
- REVIEW ROUNDS: 5

## What went well

- The owner call that bounded "deepen" by a NAMED INVARIANT LIST rather than by
  a scene or round count held for the whole task. Every scope question -
  "should thruster get a reload?" - answered itself from the roster.
- `sections_assert_their_invariant_roster` made the stopping rule executable,
  and `catalog_matches_disk` caught the merge bookkeeping for free.
- Every example was RUN headless under Xvfb, not only checked, at every step
  and every round. That is what produced the measured numbers the review could
  spot-check against, and it is what turned round 4's derived 111s deadline
  budget into a measured 21.4s.
- The YAGNI call to keep ship builders LOCAL (deferring extraction to
  `20260804-094006` as the third caller) survived review unchallenged.

## What went wrong

One defect class, filed in every single round: a step's `.until(...)` beat
reading the same quantity, against the same constant, as the `assert_*` that
follows it - which makes the assertion unfailable and turns a real regression
into a deadline stall naming the beat instead of the invariant. R1.3, R3.1,
R3.2, R4.1-R4.4: seven findings, four rounds, roughly half the branch's review
cost.

The failed decision was scoping each fix to the sites the finding cited. It
seemed sound at the time because that is what a finding's `file:line` asks for,
and because each round's reviewer had read the whole diff - if a fourth site
existed, surely the round that read all five scripts would have named it. It
did not, because a reviewer reads for defects while a sweep reads for a
predicate shape, and those find different things. R2.4 then made it worse in a
way worth naming: substituting a new quantity into a beat without re-deriving
the beat-vs-assert strength relation is how R3.2 was born out of a fix.

Round 4's response finally enumerated every `.until(...)`/`assert_*` pair
across all five scripts instead of visiting the four cited ones, and the class
closed in one pass - including two pairs (`hull_section`'s) that needed no
change, which is itself the evidence the sweep was complete.

Secondary, same root: three of the four round-1 settle-gate rewrites assumed
"N unchanged `Update` frames" implies the fixed-rate solve has run. It does not,
and the ~14fps llvmpipe verification box is the most forgiving hardware
possible for that mistake - every one of them passed locally.

## What to improve next time

- Breadth: the diff is large (4150+/1306-, 18 files) and mostly inherently so -
  five runs deepened, two absorbed, one roster test, one doc sweep. But the
  roster test is the only thing binding all five together, and it could have
  been landed last. `controller_section` + `thruster_section` (no merge, no
  deletions) was an independently landable first branch, and would have paid
  the beat-vs-assert lesson on two files instead of five.
- Churn: no plan-time question would have caught this, and that is the finding.
  The rule the fixes rest on - "a beat must be strictly weaker than the assert
  that follows it" - did not exist anywhere in the repo when the plan was
  written; this task added it (automation-harness.md:136). The plan DID say
  "gate each beat on the value it depends on rather than sleeping past it",
  which is the correct instinct one step short: it says what a beat may wait
  on and never says what a beat may not read.
- Process rule worth carrying: when a finding names a SHAPE ("the beat and the
  assert share a constant"), the fix is an enumeration of every instance of
  that shape in the diff, not a patch at the cited lines. Reply with the
  enumeration - including the instances that needed no change and why - so the
  next round can verify the class is closed rather than re-find it.
- A source-grep roster test bounds NAMES, not behavior. It proves no invariant
  was deleted into a still-green run; it cannot prove any assert can fail.
  Sabotage - the technique R3.1's response used, cutting a tolerance until the
  run panics - is what proves that, and it is cheap. Worth doing once per new
  assert rather than once per review finding.
- Context: five review rounds, each with a fresh out-of-context reviewer, kept
  the primary context flat and repeatedly caught what the implementing view had
  normalized. Cheap and worth repeating on any multi-file example task.

## Action items

- Follow-on `20260804-094006` extracts the shared ship-fixture builder as the
  third caller, per the owner call recorded here.
- Round 5's three open MINOR/NIT findings (R5.1 `LEAD_SETTLE_SECS` is inert
  because its sample lands in the prior beat, R5.2 three settle docs describe a
  stimulus-relative clock where `elapsed` counts from step entry, R5.3 one
  thruster guard doc overstates) are non-blocking doc/derivation accuracy and
  ride along with the next torpedo/thruster touch.
- The two `examples_smoke` failures round 4 recorded (`systems_`, `ui_`) are
  outside this diff and stay with the open `check/master-scenarios-flake`
  worktree.

## Landing message

```
test(examples): deepen sections/ into multi-round invariant runs

Five sections ranges, one per section family, each walking a NAMED roster of
invariants over predicate-gated rounds across as many scenes or rig layouts as
its invariants need. com_range folds into hull_section and torpedo_guidance
into torpedo_section, whose PN lead angle is now asserted rather than logged.
27 invariants total, each emitting one `outcome: <slug>` probe marker beside
its assert, pinned by sections_assert_their_invariant_roster so one cannot be
deleted into a still-green run.

Every beat waits on the value it depends on and is strictly weaker than the
assert that follows it, so a regression fails the invariant with its measured
number instead of stalling a step. automation-harness.md gains that rule and
the settle-on-the-schedule-that-writes-the-quantity rule beside it.
```
