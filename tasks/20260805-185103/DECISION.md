# Decision: Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- DATE: 20260805-185318
- STATUS: ACCEPTED
- TASK: 20260805-185103
- TAGS: tooling, testing, refactor, bcs-boundary

## Context

`NOTES.md` records a three-round read-only investigation of the screenshot and
autopilot tooling, the game crates, and the `bevy_common_systems` boundary. It
established that the examples are not hacky from laziness: each capture need hit
a missing engine capability and routed around it. The reported camera flicker is
the proof - `pose()` is correct, and the engine's camera ordering contract is
what is unfinished.

The findings are a mix of one real engine bug, four coverage/honesty defects in
the automation layer, three duplicated-capability findings against bcs, and a
cleanup tail. They are not independent: several unblock each other, and two of
them break things if done in the wrong order.

The question this decision answers is not WHETHER to fix them - the owner
accepted the finding set - but how to sequence the work and which
investigation-round conclusions to act on, given that round 3 corrected round 2
on four load-bearing points and the owner then reversed two of round 3's
conclusions.

## Decision

**One parent task carrying an eight-step ordered sequence, each step becoming
its own child at planning time.** The sequence is in `TASK.md`; the reasoning is
here.

The two ordering constraints that are not obvious, and that motivated a parent
task rather than eight independent siblings:

1. **The reel conversion precedes the probe work.** Step 5 wires `nova_timeline`
   into all six screenshot examples; two of those six are the beat-based files
   step 3 rewrites. Reversed, both files get edited twice.
2. **The prelude fix precedes the smoke delete.**
   `examples_name_drivers_through_the_nova_harness` cannot be deleted while it
   still has a subject.

Two owner rulings reverse round 3, and both are recorded as CONDITIONAL - the
conditions are steps in the plan, not assumptions:

- **`tests/examples_smoke.rs` gets deleted**, but only after probe genuinely
  covers the `screenshots/` category. Round 3 objected that probe refuses that
  category by design (`crates/nova_probe/src/catalog.rs:181-188`). The owner's
  answer is to change the design. Accepted - but the three coverage losses round
  3 documented are real, and each has an explicit fix in step 5.
- **The screenshot reel gets deleted**, but only after its two example users are
  converted to `shoot`. Round 3's withdrawal was correct while the users
  existed; converting them is a rewrite, not a cleanup, and it is budgeted in
  step 3.

Camera work (step 7) is deliberately kept OUT of the dependency chain. It
touches no automation code, and it is the highest value-to-effort item in the
investigation - it should not wait behind six steps of plumbing.

The bcs boundary rule adopted, replacing two earlier wrong versions:

> **(a) Order, don't disable.** Ordering needs only the exported `SystemSet`,
> costs one redundant write per frame, and survives bcs adding a new writer. A
> gate breaks silently when that happens.
>
> **(b) Import behavior, not presentation, and never a renderer you will not
> use.**

## Alternatives considered

**Eight independent sibling tasks, no parent.** Rejected: the two ordering
constraints above are invisible from any single task, and getting either wrong
costs rework (double-editing two example files) or a silent CI coverage hole
(deleting the smoke suite before probe covers `screenshots/`). A parent is the
only place the sequence can live.

**"Safe iff a bcs plugin exports a `SystemSet` for every system that writes
state the game also writes."** Rejected - it was checked and broke. Only `meth`
and the modding scaffold are opinion-free; orbit, PD and persist are all plugins
with schedule opinions, and every bcs module that writes shared state already
exports a set. The set is table stakes, not a differentiator. The falsifying
case is the objectives plugin (`NOTES.md`, bcs boundary section): nova adds it
for the Resource, discards the renderer, and hand-diffs `GameObjectives` to dodge
a per-frame despawn/respawn. That conflict is change-detection and renderer
ownership - no SystemSet fixes it. Hence rule (b).

**"Don't depend on anything from bcs that spawns entities."** The owner's
opening formulation. Rejected as both too strict and not strict enough: it would
exclude the camera controllers, which are fine under ordering, and it would
admit the objectives plugin, which is the one real problem.

**Keep `tests/examples_smoke.rs` permanently** (round 3's position). Rejected by
the owner. The finding it rests on - probe refuses `screenshots/` - is a design
choice, and choosing differently is legitimate. Recorded as conditional so the
precondition cannot be skipped.

**Withdraw the reel delete permanently** (round 3's position). Rejected by the
owner on merit: at a step boundary the beat list reads top-to-bottom - act,
camera, capture in source order - whereas `ReelBeat` builds the list elsewhere,
separating timing from framing. Two examples get rewritten to buy one idiom.

**Fold the perf-harness and god-mode findings into this task.** Rejected. The
owner ruled baseline storage out of scope (`git checkout <tag>` + re-run is the
policy) and ruled that a mix of god-mode and non-god-mode perf examples is
wanted. Only the narrow residuals survive, and they ride along in the step-8
cleanup tail rather than justifying their own step.

**Panic-on-step-failure as a new autopilot feature.** Rejected as already built:
`crates/nova_autopilot/src/autopilot.rs:467-484` already aborts with
`AppExit::error()` naming the step. The real gap - a step with no deadline that
hangs forever - is narrower and lands in step 8.

## Consequences

- **The flicker can be fixed today.** Step 7 has no prerequisites and needs zero
  bcs changes; every writer already exports its set.
- **The smoke delete is gated.** Step 6 cannot start until step 5 lands. If that
  gate is skipped, six screenshot examples leave CI silently - the exact
  "green and wrong" failure the investigation is about.
- **Two example files get rewritten** (`screenshot_sections.rs`,
  `screenshot_scene.rs`). That is the accepted price of one capture idiom.
- **CI grows a probe step it does not have today.** `.github/workflows/ci.yaml`
  currently runs only the smoke suite; step 6 swaps it, which means probe's
  runtime becomes CI's runtime for example coverage.
- **The prelude fix has an unmeasured blast radius.** One line at
  `crates/nova_gameplay/src/lib.rs:70`, but whatever in `examples/` and `src/`
  resolved bcs names through it will need explicit imports. Measurable by one
  build; deliberately not estimated in advance.
- **Uniform scene settle costs CI time** - roughly N examples x 1.4 s. Accepted:
  it buys CI exercising the frame that actually ships.
- **`20260731-205553` (warning cleanup) becomes tractable.** Step 2 is its front
  half; the unbounded prelude is why it has been stalled.
- **`NOTES.md` is the durable record.** The investigation's scratchpad lived in
  `/tmp/nova-invest/` and is not retained. Anything not carried into `NOTES.md`
  is lost, including the corrections list - which exists specifically so the
  four wrong conclusions are not re-derived by a later reader.
