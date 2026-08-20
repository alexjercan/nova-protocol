# What the three named spikes actually bought

Three fixes `notes-frame-spikes.md` named, landed and measured one at a time.
Every arm is headless (`NOVA_NORENDER=1`), NEVER under Xvfb, `dev` profile, on
the same host, with the two binaries saved side by side and run ALTERNATELY so
a drifting box cannot land on one arm. Each fix is measured against the fix
before it, not against master, so the numbers compose.

Ranked by TAIL, per the rule the previous round established: the mean is not
the defect, the 1% low is.

## Fix 1 - the torpedo fuze stops searching the world. WORTH KEEPING.

`torpedo_detonate_system` projected a point over the WHOLE broad phase and
declined foreign colliders in a predicate. A declined proxy returns infinity,
which never tightens the traversal's search radius, so the walk degraded toward
the entire tree per torpedo, per frame, in `Update`.

It now reads the target body's own collider list and bounds the work with the
fuze reach: a collider whose AABB is further out than the best answer so far
cannot hold the nearest point and is never projected.

Subject: `wfc_arena --ship amber --ship onyx` (a TRUE 1v1 - the default roster
is 4v4 under a capture). 15 interleaved runs per arm, 360-frame window.

| metric | before | after |
|---|--:|--:|
| 1% low (median) | 23.3 fps | **30.0 fps** |
| worst frame (median) | 57.2 ms | **41.0 ms** |
| worst frame (worst of 15) | 89.8 ms | **61.5 ms** |
| p99 (median) | 42.9 ms | **33.3 ms** |
| mean (median) | 6.71 ms | 5.90 ms |

Mann-Whitney on the 1% low: U = 56 of 225, p ~ 0.02. On the worst frame:
U = 165 of 225, p ~ 0.03.

### What did NOT work as a subject

- **The 4v4 arena.** 5 interleaved runs per arm read a 1% low of 10.77 fps
  before and 10.66 after - no separation at all, and one baseline run came in
  at a 44.2 ms MEAN. At this roster the run-to-run spread swamps the effect, so
  the 4v4 cannot measure a change of this size at any rep count affordable
  here. The 1v1 is the sharper instrument even though it is the smaller scene.
- **`stress_torpedoes` and `stress_point_defense` cannot measure this fix at
  all.** Both commit their torpedoes to a POSITION
  (`Without<TorpedoTargetEntity>`), so neither ever calls the changed function.
  They stay correctness gates, not measurement subjects.

## Fix 2 - the global scenario collision observers. WORTH KEEPING.

`area::on_collision_start_event`, `area::on_collision_end_event` and
`salvage::on_crate_pickup_play_sfx` were `app.add_observer`, so every collision
in the world dispatched into `nova_scenario` and was declined on the first
query. They are now bound to the area / crate entity itself, which is also what
makes `collider1` meaningful: avian fires the event once per side that carries
`CollisionEventsEnabled`, with that side as the target.

**The counted cause, which is the result that survives a different host:** one
`stress_point_defense` run, a scene with ZERO areas and ZERO crates, logging
the observer's own `trace!`:

| | invocations of `on_collision_start_event` |
|---|--:|
| before | **22,241** |
| after | **0** |

Frame time: `stress_point_defense`, 9 interleaved runs per arm, 900-frame
window.

| metric | before | after | |
|---|--:|--:|---|
| 1% low (median) | 119.3 fps | **146.0 fps** | U = 17/81, p ~ 0.04 |
| p99 (median) | 8.38 ms | **6.85 ms** | U = 64/81, p ~ 0.04 |
| worst frame (median) | 12.77 ms | 12.17 ms | not significant |
| mean (median) | 1.894 ms | 1.826 ms | not significant alone |

The mean moves 0.07 ms, which is the 56.5 us/frame the trace attributed to
these two observers arriving where it was predicted - but on its own it is
inside the noise (U = 54/81).

**NOT measurable on the arena.** 10 interleaved runs per arm; after discarding
the windows that held no fight (below), every metric came back |z| < 0.6 with
the medians on top of each other. The arena's own spread is wider than what
this fix removes.

## Fix 3 - the entity-speed sampler is gated on a reader. NOT MEASURABLE.

`sample_scenario_queries` walked every `EntityId` carrying a `LinearVelocity`
and allocated two `String`s per match, every frame, whether or not anything
could read the result. Its cost scales with the WORLD - severing turns each
hull section into another free body - and not with the scenario.

| subject | runs per arm | result |
|---|--:|---|
| `stress_point_defense` | 9 | every metric \|z\| < 0.7 |
| `wfc_arena` 1v1, fight windows only | 8 | every metric \|z\| <= 0.53 |

The trace attributed 24.1 us to this system on a 12 ms frame - 0.2% - and a
repeat set of this size does not resolve 0.2%. **The frame-time claim is
withdrawn; the code is kept for the COUPLING it removes**, which is the fourth
unbounded surface `notes-frame-spikes.md` listed: entity count is the one input
no authored content controls, and this was the system that turned it into
per-frame cost. If the owner wants the diff smaller than the benefit, revert it
- the numbers do not defend it.

**The gate is NOT "a declared watch", and that matters.**
`/create/expressions/` documents an entity query as an inline expression factor
too:

```ron
VariableSet((key: "speed_at_gate", expression: Term(Factor(Query(Entity(...))))))
```

An inline query is answered from the same snapshot and has to be available the
FIRST time its action runs - an expression over an unavailable value fails
closed - so a watch-only gate would have silently broken that authored surface.
`ScenarioConfig::reads_an_entity_query` reads the watches AND the inline
factors, once at load. The inline walk is the one the lint already did; it
moved onto `ScenarioConfig` so both readers share it and cannot drift.

No shipped scenario reads an entity query at all, so the sampler is dead work
in every one of them today.

## `SPAWN_DRAIN_BUDGET` - REJECTED, and the check placement is not the defect

The claim under test was that `SPAWN_DRAIN_BUDGET` is checked AFTER the
command, so a frame overruns. Measured directly: a temporary counter on the
drain loop, one headless 4v4 `wfc_arena` load, logging `(commands applied,
frame elapsed, worst single command)` per drain frame. 46 drain frames:

| | frames |
|---|--:|
| applied exactly ONE command | **38 of 46** |
| applied 2 | 4 |
| applied 6-13 (the rock scatters) | 4 |

On every one-command frame `elapsed == worst_command` to three decimals. The
tail is **28 consecutive frames of one 11-17 ms command each** - 8 hulls plus
20 derelict fragments, exactly the objects the scenario authors. Worst single
command **17.27 ms, 5.7x the 3 ms budget.**

So the overrun IS one command, and no placement of the check can prevent it:
the loop's first iteration always runs, which is what stops the drain
deadlocking on an object that costs more than the whole budget. The one frame
where placement could have helped applied 7 cheap commands (~0.35 ms) before a
13.9 ms one - 2.4% of that frame, once in 46.

Fixing this needs one authored hull's spawn SUBDIVIDED across frames, and the
drain's own doc records why it is one command: a ship's sections all land
inside one `apply`, so the `Added<SectionLinkPoints>` batch the integrity graph
and the derived skin key off is complete the first time they see it.
Subdividing means giving those consumers a partial hull, or a new "hull is
complete" signal for them to wait on. That is a design change, not a budget
tweak, so it is left where it is with the number that sizes it.

## Two instrument findings, both of which cost real time here

### `probe run` slurps and DOM-parses `trace.json`. It OOMed a 31 GB box.

`RunArtifacts::load` (`crates/nova_probe_cli/src/evaluation/artifacts.rs`)
reads every artifact with `std::fs::read_to_string`, then
`aggregate_system_costs` (`evaluation/profile.rs:96`) does
`serde_json::from_str::<serde_json::Value>` over the whole string. Both the
text and the DOM are live at once, and a `serde_json::Value` DOM runs several
times the size of its text.

`probe run system_torpedo_launch --norender` - one SMALL range - wrote a
**7.35 GB `trace.json`** and the probe HOST process reached **27.8 GB RSS**
against 31 GB of RAM and 1 GB available. It had to be killed by PID. Nothing in
the loader looks at the file size first.

This silently caps how much anyone can profile: the frame-spikes lane's own
arena trace was 4.9 GB, so that run was already within a factor of two of the
same wall. Use `--correctness-only` for a behaviour gate - it runs the clean
pass only, no capture and no trace - and do not pass several examples to one
`probe run` until the loader streams.

### The arena's capture window sometimes holds no fight

Nine runs across the arena A/B sets came back with a ~2-3 ms mean, a worst
frame under 8 ms and a 1% low of 140-175 fps - an order of magnitude off the
rest. That window opened on `fight_happened` and then measured the aftermath,
not the fight. It is a property of the SUBJECT (both arms drew them), but it is
bimodal rather than merely noisy, so a repeat set has to discard it explicitly:
here, any run whose worst frame is under 10 ms. One fix2 arena set drew four of
them in one arm and none in the other, which was enough to make a fix that
removes 22,241 calls a run look SLOWER. That set was thrown away.

`capture_simulated` does not catch this - the simulation is running, there is
just nothing left in it to fight.

## Measurement hygiene, and what was discarded for it

- All arms interleaved, one process at a time, never two measurements at once.
- Two sets were taken while the box was NOT quiet - an unrelated
  `nix build` and, later, `opencode`/`plannotator` starting up - and both were
  DISCARDED and re-run: the first fix2/fix3 round entirely, and the fix2 arena
  set (which also carried a 283 ms frame no mechanism in the diff can explain).
- `probe run` writes an instrumented binary to the same path `cargo build
  --example` does, so every arm here ran from a COPY saved before any probe
  pass, never from `target/debug/examples`.

## Changelog

Fix 1 gets NO changelog entry. `distance_to_skin` and its whole-world search
landed on 2026-08-18, after v0.10.0 shipped on 2026-08-13, so it is a thing
added and fixed inside one cycle and the reader's baseline never saw it. The
contact-fuze entry already in `[Unreleased]` describes where it ended up.
Fixes 2 and 3 touch code released in v0.10.0 or earlier and get one entry each.
