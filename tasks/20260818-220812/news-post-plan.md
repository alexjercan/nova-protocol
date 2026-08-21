# The v0.11.0 news post: plan, and the media it needs

Owner asked for a Factorio-style release post: things moving, mechanisms
explained, numbers on the table. This is the plan and the capture manifest.
It is not the post - `web/src/news/0.11.0.md` is, and it already holds 759
lines of drafted detail.

## Who reads it

A player or a follower, not a developer. The dev book already owns mechanism
for developers; `/create/` owns the authored contract for modders. This post
owes neither. It owes: **what changed about playing the game, shown.**

The reader's baseline is **v0.10.0**, not last week. Anything added and then
revised inside the cycle gets ONE description - where it ended up. Anything
added and removed inside the cycle never happened.

## What the draft has and what it lacks

`web/src/news/0.11.0.md` is organised by the CHANGELOG's ten groups, which is a
CHANGELOG structure, not a story. It is a good body and a bad spine.

| group | entries | in draft |
|---|--:|---|
| Internals & Tooling | 63 | yes, and mostly not post material |
| Combat & Weapons | 34 | yes, strongest section |
| Ships & Sections | 30 | yes |
| Fixes | 30 | yes |
| Gameplay & Flight | 23 | yes |
| Interface & HUD | 17 | yes |
| **Performance** | **14** | **MISSING - no heading at all** |
| Modding & Mod Portal | 9 | yes |
| Audio & Visuals | 9 | thin |
| Scenarios & Objectives | 8 | yes |

Two structural changes:

1. **Add a Performance section.** It is the one group with no heading, and it
   is the group with the best material.
2. **Put a lead in front of the groups.** Five things a player will feel,
   each with a moving image. The grouped body stays as the scannable rest.

## The lead: five things, five loops

Ordered by how visible the change is when you play.

1. **Ships come apart.** Cracks deepen as a section takes damage, turrets spark
   when they break, dead sections sever into wrecks that drift. This is the
   most visible single change in the release.
2. **Point defence is real.** Each mount picks the most imminent torpedo it can
   bear on, fires inside a 0.1 degree cone, and leads a filtered track. The
   counter is that torpedoes now weave to break the lead solution.
3. **A round's identity is how it travels.** The resistance table is gone.
   Kinetic is a punch that carries on only through what it destroys; Pierce is
   a rake that crosses every section on its line.
4. **Ships have a skin.** Cladding derived from structure, four styles, and
   greebles that stand up straight on the plate they sit on.
5. **Gunfights moved in close.** 1-2 km instead of 4-5. Everything is legible
   now because everything is nearer.

## The performance section, and what it may honestly claim

The measured position, as of 2026-08-22. Read `perf-check-2026-08-21.html` and
`DECISIONS.md` D22 for the workings.

### The headline is STRUCTURAL, not temporal

**A thousand bullets in the air used to be a thousand rigid bodies and a
thousand colliders. Now it is neither.** Measured by archetype census on both
trees, same driver, same scene:

| | rounds in flight | rigid bodies | colliders |
|---|--:|--:|--:|
| v0.10.0 firing | 1000 | 1035 | 1046 |
| v0.11.0 firing | 400 | **35** | **46** |

Caveat that belongs in the post: the volumes differ because turret reach went
500 -> 200u and round lifetime 5 s -> 2 s. Part of the lighter scene is content.

This is the right headline because it is verifiable, it does not depend on a
host, and it is the shape a reader can picture.

### The paired wins, each with a before and an after

| what | subject | result |
|---|---|---|
| Damage cracks quantised to 8 buckets | gallery | 0.592 |
| Placeholder art built once, not per entity | gallery | 0.631 |
| Exhaust plume written only on change | frozen gallery | 0.733 |
| Thruster + torpedo materials | PD stress | `min_ms` 17.83 -> 5.47 |
| Fixed loop single-threaded | arena duel | 1% low 27.09 -> 47.53 fps |
| A round is swept math | PD stress, per step | worst step 17.4 -> 11.8 ms |
| A round is swept math | PD stress, per frame | worst frame 71.08 -> 44.93 ms |
| Owner hand-run, 17 hulls | gallery | 7 -> 16 fps |

### What the post may NOT claim

- **No single "the game is N% faster".** The cross-release subject
  (`broadside`) is render-bound, not sim-bound: 1.59 fixed steps a frame firing
  against 1.55 idle, so deleting a thousand bodies barely changes how much
  simulation runs. Its mean is flat. The wins are real and they are on
  sim-saturated subjects.
- **No player-facing frame time from a dev build.** Dev RANKS, release
  CERTIFIES: first-party code at `opt-level = 1` against dependencies at `3`
  exaggerates our own cost. Every number taken so far is dev. If the post
  quotes a frame rate, it owes a release-profile pass first.

### The wart, and I recommend publishing it

`broadside`'s p95 is **+8.8% worse** than v0.10.0's, 6/6 pairs the same sign,
reproducing across five passes. Under test now: v0.11.0 renders 65 ship sections
through a damage-cracks extended material that v0.10.0 does not have, so an
identically-sized scene draws through two pipelines where it drew through one.

One honest line about it buys more credibility than the rest of the section
combined, and the trade is the kind the epic's rules already call negotiable -
cracks are presentation. **Owner decides.** Wait for the A/B before writing it.

### Memory

Peak RSS **2.27 -> 2.30 GiB** across the release. The first memory figure this
project has ever had; worth one sentence, because "we added all of this and it
costs the same RAM" is a real result.

## Media manifest

Two pipelines already exist and both work. **Nothing here needs new tooling.**

- **Loops**: a `loop_*` example in `examples/screenshots/` records between its
  `loop_start`/`loop_end` autopilot steps and encodes its own webm;
  `scripts/capture-web-media.sh` stages, runs, validates and copies into
  `web/src/assets/loops/`.
- **Stills**: a `screenshot_*` example shoots; `scripts/gen-web-screenshots.py`
  packages it against a SLOTS row. Naming follows `news-0110-<slug>.png`, as
  `news-090-*` did.

### Loops - 2 exist, 7 wanted

| loop | shows | status |
|---|---|---|
| `spine-cut` | a flank cut through, engine block severs and drifts | **exists** |
| `torpedo-blast` | a warhead detonating on a hull | **exists** |
| `loop_hull_cracks` | one section walked down through all 8 crack buckets under held fire | NEW |
| `loop_turret_sparks` | a turret taking damage, sparking, breaking, going quiet | NEW |
| `loop_point_defense` | a salvo inbound, mounts assigning per turret, tracers, intercepts | NEW |
| `loop_torpedo_weave` | Serpent corkscrewing beside a Lance running straight | NEW |
| `loop_pierce_vs_kinetic` | same shot, same target, both round types - punch stops, rake carries | NEW |
| `loop_skin_styles` | one hull re-skinned across industrial / armoured / civilian / salvage | NEW |

The first two of the new ones are exactly what the owner asked for. Three, four
and five are the release's combat identity and none of them can be shown in a
still.

### The perf visual worth building

**A side-by-side of the same firefight with collider gizmos on: v0.10.0 drawing
a thousand collider wireframes, v0.11.0 drawing none.** Both worktrees are still
on disk with the driver in place, and `--features debug` has the gizmos. This is
the single most Factorio-shaped image available and it makes the headline number
self-evident.

### Stills - `news-0110-*`

| slot | shows |
|---|---|
| `news-0110-crack-buckets.png` | one hull at 8 damage levels, strip |
| `news-0110-skin-styles.png` | four styles side by side |
| `news-0110-editor-skin.png` | the editor showing derived cladding |
| `news-0110-parts-gallery.png` | the gallery picker with previews |
| `news-0110-menu-carousel.png` | the scenario carousel |
| `news-0110-greebles.png` | the greeble vocabulary |
| `news-0110-wreck-field.png` | after a fight: hulls opened, wrecks drifting |

**One of these is already free.** `examples/screenshots/screenshot_damage_levels.rs`
stands five identical clad ships in a row at progressively worse damage and
exists specifically so a human can judge the looks - and it has **no SLOTS row**,
so nothing packages what it shoots. `news-0110-crack-buckets.png` is one line in
`scripts/gen-web-screenshots.py` plus a run.

`screenshot_editor`, `screenshot_hero_ship` and `screenshot_scenario_picker`
already have slots but are aimed at other surfaces (`feature-`, `wiki-`,
`news-090-`); reusing a shot across surfaces is fine, minting a near-duplicate
is not. Check each before writing a new example.

## Queue, and what gates what

The GPU is ONE lane. Nothing here may overlap another measurement.

1. **Cracks A/B** - running. Prices the p95 regression, or exonerates cracks.
   Gates the performance section's honest wart.
2. **Pipeline experiment, arm A** - one command, no code change. Settles
   whether a shader compile blocks a frame outside the loading screen. Owner has
   already decided the fix if it is real: **compile in the loading screen.**
3. **Shader pre-warm**, only if arm A confirms. `PipelineCache::process_queue`
   is `pub` and documented as callable manually; that is the hook. A wgpu disk
   cache is NOT available - bevy hardcodes `cache: None` and the feature is
   Vulkan-only.
4. **Release-profile pass** - required only if the post quotes a frame rate.
5. **Loop and still capture** - last, so every image shows the shipped build.

## What this post should not do

- Not a changelog. 237 entries do not go in it; the CHANGELOG has them.
- Not the dev book. Mechanism goes to `docs/`, the authored contract to
  `/create/`. A player cannot run `probe run`, so no harness detail.
- No number tuned to read better. Every game number is lifted from the Rust
  source with its `file:line` on the comment, per the Web conventions.
- No performance claim without a before and an after.
