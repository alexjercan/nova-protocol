# Split the examples three ways: playable, systems, screenshots

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,example,docs

Epic: `20260818-220812`. Owner: "I want some examples where both the player and
autopilot can play them: e.g I want to be able to carve an asteroid by hand,
but also have the autopilot do it for screenshots/gif; but there are a lot of
examples that only work for autopilot, some of which are fine ... but there are
some which don't do anything if I load them as a player".

Owner, 2026-08-19, setting the shape: a THREE way split, not two.

## The three categories

An example is sorted by **who it is for**, not by what it happens to be able to
do.

1. **`playable`** - made for a HUMAN. A person loads it and does the thing it
   demonstrates. It MAY also carry autopilot, and often should: the autopilot
   is a second driver for captures and gates, never the only one. Owner's
   example: the greeble gallery belongs here, and it is still worth having
   screenshots of.
2. **`systems`** - AUTOPILOT ONLY. Correctness. Reproduces a found bug or pins
   a system already under test - sections, integrity, old regressions. Never
   playable, and nobody should expect to load one by hand.
3. **`screenshots`** - AUTOPILOT ONLY. Its output is IMAGES. A scripted shot
   needs a scripted camera, and making it playable would break the shot.

The test between `playable` and the other two is: **would a human loading this
expect to do something?** If the name promises a verb, it owes the verb.

The test between `systems` and `screenshots` is what the run PRODUCES: an
assertion, or a picture.

An example that silently does nothing when a human loads it is the defect being
fixed. After this task, that state does not exist - it is either playable, or
it is declared as autopilot-only where a human sees the declaration before
loading it.

## This CHANGES a convention

`CONVENTIONS.md`, Nova rule 1, currently ends: "`systems/` is where correctness
lives; `screenshots/` is where content does. **There is no third category**, and
a range that only measures is not a range."

That rule is superseded by this task and must be rewritten IN THE SAME CHANGE as
the tree move - the docs routing rule applies to `CONVENTIONS.md` like anything
else. Keep the part that still holds: a range that only measures is still not a
range, and `systems` still owns correctness.

## The move

`examples/playable/` is a new directory beside `examples/systems/` and
`examples/screenshots/`. Sorting an example into `playable` means MOVING the
file, not tagging it.

Watch for things that break on a move, because ids here are runtime strings and
nothing type-checks them (`CONVENTIONS.md`, Nova rule 3):

- the roster in `crates/nova_probe_cli/tests/catalog_drift.rs`
- `probe run <category>` and whatever enumerates categories
- `Cargo.toml` example paths
- CI workflow invocations
- any doc naming an example by path

## Audit first

`examples/screenshots/` (22) and `examples/systems/` (23). For each: load it as
a human, and record which of the three it is and what it would take to make it
playable if it is close. **The audit table is the deliverable of the first
pass** - do not start moving files before the list exists and the owner has seen
it.

Known calls, from the owner:

- `carve_asteroids` -> **playable**. Carving a rock by hand is the thing the
  whole destruction epic shipped and right now it can only be watched.
- `greeble_catalog` -> **playable**, keeping its captures.
- `screenshot_combat` -> **screenshots**. Explicitly fine as a rig.

Strong candidates worth checking early, not decided: `wfc_arena`,
`parts_viewer`, `thruster_gallery`, `damage_levels`, `widget_zoo`.

Note the overlap with `20260819-012153` (scenario coverage): that task asks
whether the automation loads what PLAYERS load, this one asks whether a human
can load what the automation runs. Same audit from opposite ends. Do them
together or do this one first, but do not do them independently and reconcile
later.

## Rules for the conversion

- Examples doubling as gates KEEP their gates. Playability is added alongside
  the assertions, never instead of them.
- Do not convert rigs for symmetry. A scripted shot that a human cannot
  meaningfully fly stays a rig.
- The description is the surface a human reads. Whatever lists examples must
  show it - check that the description actually reaches a player and fix it if
  it does not.

## Done when

- The audit table exists in this task, every example in one of the three.
- `examples/playable/` exists and every example in it has been loaded BY HAND
  and played - verified by loading, not by reading the code.
- Every `systems` and `screenshots` example says in one line what it is for and
  that it is autopilot-only.
- `CONVENTIONS.md` Nova rule 1 describes the three categories.
- `catalog_drift` green, `probe run` still finds everything it did before.

## The audit

Read STRUCTURALLY, from the source, not by hand-playing. What is established
per example: whether it spawns a player-controlled hull with the flight verbs
bound, whether an interactive key handler is registered OUTSIDE the
`NOVA_AUTOPILOT` gate, and whether the run's product is an assertion or a
frame. Where the entry says "structurally playable" it means exactly that -
the affordance is wired in a plain run. **Nothing below was hand-verified as
FUN or even as usable.** Final playability stays the owner's call.

Two facts hold for every example and are not repeated per row:

- `AutopilotPlugin` is inert without `NOVA_AUTOPILOT`, so a plain
  `cargo run --example X --features dev` never runs the script.
- The scenario loader gives the scenario camera a `WASDCameraController` and
  takes it away only when a player hull exists. So EVERY example without a
  player ship already has a free-fly camera in a hand-run. A free-fly camera
  alone is not an affordance - it is how you look at a rig.

### playable (10) - all moved out of `screenshots/`

| example | why | evidence |
| - | - | - |
| `carve_asteroids` | owner's call, and it holds up | player hull with `input_mapping {"pdc": LMB}` and `infinite_ammo`; a hand-run flies the rig and shoots the row |
| `greeble_catalog` | owner's call, and it holds up | `keyboard` system registered in `catalog_plugin` (unconditional): arrows, Enter focus, `L`/`C`/`G` |
| `parts_viewer` | a browser, and the whole file is the browsing | unconditional `keyboard` in `Update`: arrows, PageUp/Down, Enter, Tab, `X`, Esc across three views |
| `widget_zoo` | every widget is live and clickable | `NovaUiPlugin` + widget observers + `toggle_skin_key`, all unconditional; the doc calls it "live, FUNCTIONAL" |
| `wfc_arena` | the strongest case in the tree | `--ship TEAM:player` spawns YOU under the campaign controller; lobby, pause menu, result board, `Q`/`E`/`1-4` vantages. REFUSES a player slot under `NOVA_AUTOPILOT` |
| `wfc_ships` | `R` re-rolls the row | `reroll_on_key` in `wfc_plugin` (unconditional): `R`, `C`, `L` |
| `shape_bench` | a bench you flip through | `restyle_on_key` in `bench_plugin` (unconditional): `L` cycles the look, `C` strips cladding |
| `block_bench` | same shape as `shape_bench` | `restyle_on_key`, same two keys |
| `compare_asteroids` | the comparison IS the hand-run | `compare::select_by_keys` (unconditional): digits 1-9 and arrows re-dress the focus subject |
| `compare_planets` | same kit, same keys | `compare::select_by_keys` through `shared/compare.rs` |

### screenshots (12) - stay

| example | why | how close to playable |
| - | - | - |
| `screenshot_combat` | owner's call. Product is the travel/combat/HUD image set | structurally playable (player corvette, and the ambush is scenario data so a plain run gets it) - but the scripted camera cut IS the example |
| `screenshot_flight` | product is three manifest images; the camera flies the leg | same shape as `screenshot_combat`: player racer, scripted flight, posed camera |
| `screenshot_scene` | posed beauty set, no input at all | not close - nothing to do but look |
| `screenshot_sections` | five posed closeups, ship yaws to the camera | not close |
| `screenshot_ui` | drives the SHIPPED menu/editor to capture it | a hand-run gives you the real game menu, which is the game, not this example |
| `screenshot_nova_os` | shoots the terminal and the ship app | the computer itself is playable in the game and in `systems/nova_os` |
| `loop_torpedo_blast` | product is a webm | every actor scripted or inert, by design |
| `loop_spine_cut` | product is a webm | same |
| `render_scale_shot` | product is one PNG at a preset | boots a shipped scenario, so a hand-run is playable - but the example is the PNG |
| `scene_baseline` | product is a frame-time number | env-var driven measurement rig; a range that only measures is not a range |
| `thruster_gallery` | **checked on the owner's list**: a posed row with NO key handler at all | CLOSE. `gallery_plugin` registers only `frame_new_camera`, `place_labels`, `stop_orbit_on_input`. Give it `greeble_catalog`'s selection ring + focus turntable and it moves |
| `damage_levels` | **checked on the owner's list**: five posed ships, no keys, no player hull | CLOSE. Needs either the same selection layer or a player rig that re-damages a column by hand |

### systems (23) - all stay

Every one is on the `SYSTEMS_ROSTER` in
`crates/nova_probe_cli/tests/catalog_drift.rs`, and
`systems_ranges_assert_their_invariant_roster` asserts roster set == `systems`
category set. Moving one out costs its gate, which the conversion rules forbid.
Their audience is the probe.

`attitude_hold`, `thrust_and_plume`, `hull_damage`, `destruction_finale`,
`turret_gunnery`, `torpedo_launch`, `blast_penetration`, `section_severing`,
`scenario_grammar`, `player_path`, `outcomes`, `neutralized_quiet`,
`borrowed_battery`, `ship_editor`, `sandbox_soak`, `hud_indicators`,
`menu_boot`, `menu_picker`, `nova_os`, `stress_bullets`, `stress_torpedoes`,
`stress_one_structure`, `stress_many_structures`.

THREE of them are genuinely dual-use and the owner should know:

- `turret_gunnery` - the player hull binds Space/RightTrigger to the turret, so
  a hand-run tracks and fires a real gun at the sweeping gate.
- `torpedo_launch` - same binding, plus the guidance gizmos and the trail the
  doc calls "the interactive harness for the torpedo work".
- `hull_damage` - `KeyO` spins the rig and `KeyK` kills a section by hand.

They stay because their product is a verdict and their roster is the gate.
Moving any of them means re-plumbing
`systems_ranges_assert_their_invariant_roster` off the directory and onto the
roster list itself. That is a real option, it is just not free.

The rest of `systems/` builds its rig in a plain run (setup is in
`OnEnter(GameAssetsStates::Loaded)` or `Startup`, never inside the script), but
the BEHAVIOUR under test is what the script drives. A human loading
`blast_penetration` gets three rigs and no blast.
