# Perf baselines across the example suite, 4v4 as the benchmark

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.11.0,performance,harness

Epic: `20260818-220812`. `PERF-BENCH`.

**This task builds an instrument. It does not promise to make anything faster.**
Fixes are separate tasks that each cite a baseline and show a delta - which is
what stops the pattern that killed `20260818-221031` and `20260818-221036`, both
ranked at p80+ against costs that had already moved.

## The problem

Two, and the second is the blocker.

**We can only measure what an example covers.** `20260819-123928` made `probe
scenario` work, so a scenario no longer needs an example - but a SUBSYSTEM still
does. Nobody knows which subsystems are exercised by what runs today. Owner and
assistant guessed opposite answers about whether the thruster balancer is even
touched by a 4v4, and neither had checked.

**Worst-frame cannot currently prove an improvement.** Measured, unchanged
scene, twice: `editor_sandbox` 57.87 ms then 42.38 ms; `broadside` 58.38 then
52.92. That is ~30% run-to-run on the tail, so a 25% win is invisible and a 25%
regression reads as noise. Means were stable to 0.5%.

## The metric - owner's design

Report mean AND median AND worst, and use the stable statistics as a VALIDITY
GATE on the run rather than as the score:

- If a run's mean/median drifts from the baseline's, the run was contaminated -
  background load, thermal, another probe - and is DISCARDED, not averaged in.
- Worst frame is then read only on runs that pass the gate. It stays the number
  that matters, because a stutter is a tail and a mean hides it.

This converts the noise problem from "worst-frame is unusable" into
"worst-frame is usable on runs that pass". Write down the gate's tolerance and
the repeat count, because every later claim rests on them.

## No release profile

Decided. A dev build does not hide a bad algorithm, an unbatched draw or a
per-frame material rebuild. It also handicaps FIRST-PARTY code specifically -
`opt-level = 1` against `3` for dependencies - so our systems are exaggerated
relative to bevy and avian, which makes dev the better instrument for finding
OUR problems. The 4v4 ranking already showed no Nova system above 21 ms while
our code was the handicapped half; release only widens that gap.

Release matters for CERTIFICATION - "does it hold 60" - not for ranking. Take
one release run at the end if a frame-rate verdict is ever claimed.

## Steps

1. **The metric.** Gate tolerance, repeat count, which statistics are recorded.
   Everything below depends on it.
2. **Coverage map.** For each existing example and scenario, which subsystems it
   actually exercises - MEASURED, not assumed. Autopilot, thruster balancer,
   targeting, carving, NOVA OS, editor, weapons, WFC generation. Output is a
   table with holes in it.
3. **Fill the holes** with new `stress_*` cases, one subsystem each. The
   category already exists: `stress_bullets`, `stress_torpedoes`,
   `stress_one_structure`, `stress_many_structures`.
4. **Record baselines**, re-runnable and comparable. `wfc_arena` 4v4 is the
   headline benchmark: it is the only case that puts the whole core under load
   at once, and at 295.76 ms worst it is six times worse than anything else
   measured.

## The four 4v4 candidates, to CONFIRM or REJECT against the baseline

From `20260819-123928/NOTES.md`. A rejection is a result here.

1. **Fixed-timestep amplification.** `run_fixed_main_schedule` is 138.43 ms, the
   second-largest span, and unexamined. With `Time<Virtual>::max_delta` at 0.25 s
   against a 1/64 s step, one slow frame runs up to 16 fixed steps, making the
   next frame slower. If the 295 ms tail is partly a spiral, capping `max_delta`
   bounds it WITHOUT making anything faster - and changes what every other
   number means. Cheapest check, so it goes first.
2. **Render schedule**, 188.77 ms worst, 27.5% of the traced run.
   `write_binned_instance_buffers` 65.98 ms and
   `prepare_preprocess_bind_groups` 16 ms/call both scale with distinct binned
   mesh entities. COUNT them before proposing a fix - meshes, materials and
   entity count want different answers.
3. **`collect_collision_pairs<ProjectileHooks>`**, 59.87 ms, the biggest
   Nova-attributable call. A round is a physics body, so a thousand rounds is a
   thousand BVH entries. That is a design question, not an optimisation.
4. **`ThrusterExhaustMaterial` re-prepared every frame** - 5.9 ms/call, 673 ms
   over 114 frames. Pure waste and probably the cheapest real fix. Only 1.5%
   of the 4v4, so it barely moves THAT case - but owner suspects it dominates
   `stress_torpedoes`, which is a second win in a different case. Confirm
   separately rather than folding it into the 4v4 number.

## Out of scope

- `spawn_ship_skin`, 20.77 ms at spawn - that is load cost and belongs to
  `20260818-221040`.
- Any fix. This task confirms or rejects; fixes are filed separately.

## Done when

- The metric is written down with its gate tolerance and repeat count.
- The coverage table exists, with every hole either filled by a `stress_*` case
  or named and left with a reason.
- Baselines are recorded and re-runnable, 4v4 headline.
- Each of the four candidates is landed as confirmed, or REJECTED with the
  measurement that rejected it.
