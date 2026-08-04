# Retro: Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

- TASK: 20260804-094006
- BRANCH: examples/stress-category
- REVIEW ROUNDS: 1

## What went well

The plan carried literal, `rg`-derived file:line lists for the rename sweep
(seven string sites, six wiki lines) instead of "update the references". Every
one of them was delivered and the review's step-by-step re-read found no
undelivered clause - one APPROVE round with one MINOR and three NITs, and zero
rework cycles.

Deferring the fixture extraction until a third caller existed paid off. The
owner note in `torpedo_section.rs` held the signature open until three real
shapes were visible, so `fixtures::ship`/`asteroid`/`spawn_on_start` was
designed from evidence and landed with six callers on day one. The two
signature parameters that went beyond the plan's literal text (`controller`,
`lock_signature`) exist because callers demanded them, not because they were
anticipated.

Every sweep asserts as well as measures. A frame-time-only category would let a
leak show up as slow drift; the teardown-to-baseline panics, each with a paired
delivery guard (`swarm_is_up`, `structure_is_up`, `field_is_up`,
`MIN_PEAK_ROUNDS`), turn that into a `cargo test` failure.

## What went wrong

**The count knob measured nothing.** `many_bodies` first scaled `SHELL_RADIUS`
with the cube root of the count so inter-rock spacing stayed constant. That was
a defensible instinct - it keeps the scene visually comparable across counts -
but it holds DENSITY constant, so rocks per frustum and per broad-phase cell
barely move and the cost saturates: 400 and 800 rocks both measured ~40 ms.
Pinning the radius put the count back into the number. The general shape: a
scale sweep must vary the thing the systems are quadratic in, and "keep the
scene comparable" is the instinct that quietly removes it.

**A silently dropped hook cost a full measurement cycle.** Two chained
`.on_enter(...)` calls on one `ScriptBuilder` step: the second replaces the
first rather than appending, so `capture_reload_end` never ran, the reload gate
latched open, and every frame after the first loop was excluded. Nothing
failed - the run passed with an empty capture. The mitigation that shipped is
one closure doing both jobs plus a warning comment repeated on all three
sweeps, which protects these three callers and nobody else.

**A silently dud turret.** Pressing fire while the weapons safety is still cold
latches nothing, and a held key produces no fresh edge once it goes hot, so the
volley never fired and the frame time looked honestly fast. Fixed with an
explicit `WeaponsHot`-gated raise step, a `cease_fire` that RELEASES so the next
loop cycle gets a real edge, and `MIN_PEAK_ROUNDS` as a dud detector. The
pattern behind all three: a perf example that measures less work than intended
reports a BETTER number, so it has no natural failure mode - every stress run
needs a floor assertion on the stimulus, not only on the outcome.

## What to improve next time

**Breadth.** 19 files, ~1760 insertions. Not a missed split: the category move,
the fixture extraction and the three sweeps are one contract change (`stress/`
becomes the sole frame-time home), and the plan's numbered groups landed as four
clean commits in that order. The extraction was explicitly parked here by an
earlier owner call. Repeat the structure - plan the commit boundaries, not just
the work.

**Churn.** Effectively none. The one MINOR (`many_bodies` missing the `.max(1)`
its two siblings carry) is the kind of inconsistency the plan could have
foreclosed by writing the knob's contract once, in the Steps, instead of
per-example. When N examples share an env var, specify the parse-and-clamp once
and have each example match it.

**Context.** No checkpoint, compaction warning or handoff is recorded for this
task, and the work landed as an in-order commit sequence, so nothing to split or
defer on those grounds. The one real cost was measurement wall-clock: picking
the three `DEFAULT_COUNT` values needed actual sweeps (the throwaway script paid
for itself twice), and `probe run stress --fps` is a ~15-minute proof. Budget
for that up front rather than treating it as a final check.

## Action items

- `20260805-015136` (backlog, bug): fix `ScriptBuilder::on_enter` to append or
  reject a second call, and delete the three duplicated warning comments.
- `20260805-015146` (backlog, examples): clear review findings R1.1-R1.4,
  including the `many_bodies` `.max(1)` clamp.

## Landing message

```
feat(examples): create stress/ as the single frame-time category

Absorbs perf/perf_baseline as stress/scene_baseline (a pure move: every
NOVA_PERF_* name and default is preserved, so the release-over-release
number stays comparable) and adds three scale sweeps - many_bodies,
many_sections and many_projectiles - each with a NOVA_STRESS_COUNT knob, a
loop_from point so --fps measures activity rather than an idle tail, and a
panicking teardown-to-baseline assertion so every run makes a correctness
claim alongside its number.

Drops the TRANSITIONAL perf catalog row; category policy now decides which
runs carry an fps pass. The SpaceshipConfig/asteroid builders that
sections/ and systems/ had been copying inline become nova_probe::fixtures
with unit tests, and six examples now call them.
```
