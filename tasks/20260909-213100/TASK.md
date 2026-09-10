# Split the probe matrix into balanced shards

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.14.0, ci, probe, tooling

## Goal

Shorter CI on every push, so the review-heavy cycle gets its verdicts faster
and adding systems ranges does not lengthen the critical path. Owner
(2026-09-09): "we can do more splits to have easier CI, so we parallelize
more."

## Measured (run 34317217860, 2026-09-09, warm cache, all green)

| job | wall | of which build | examples |
|-|-|-|-|
| probe / screenshots | 51 min | 6 min | 37 |
| probe / systems | 35 min | 6 min | 43 |
| probe / playable | 28 min | 6 min | 17 |
| fmt / clippy / test | 21 min | - | - |

The screenshots shard is the critical path. Its slowest ranges:
screenshot_editor 187 s, screenshot_gravity 155 s, loop_vfx_range 146 s,
screenshot_railgun 140 s, loop_round_types 127 s, loop_command_shell 103 s,
screenshot_menu 102 s. Systems: system_ship_editor 226 s,
system_attitude_hold 153 s, stress_torpedoes 139 s, stress_hull_collapse
132 s, stress_point_defense 112 s, stress_one_structure 99 s. Playable:
first_shift_ships 222 s, planet_types 168 s, carve_asteroids 151 s.

Measured run time sums to about 70 percent of each sweep; the rest is
probe compiling and launching each example in turn.

## Plan

No new probe flag is needed: `crates/nova_probe_cli/src/native/spec.rs`
already resolves `probe run <example>[,<example>...]` beside a category
directory. The split is yaml plus the guard step.

1. Replace the three directory shards in `.github/workflows/ci.yaml` with
   four SUBJECT shards of about 25 examples each, so a red shard says
   "combat is red" instead of "screenshots is red", and every new range has
   an obvious home:
   - `probe / ui`: the editor and menu walks, the NOVA OS ranges, the
     headless spikes, `bug_sandbox_soak`, `widget_zoo`, `parts_viewer`.
     Carries both editor outliers (`screenshot_editor`,
     `system_ship_editor`), so it is the shard to watch.
   - `probe / combat`: the combat and lock screenshots, the weapon loops,
     the gunnery, torpedo, railgun, blast, HUD indicator and line-of-sight
     ranges, `railgun_wake_bench`, `wfc_arena`, `wfc_ships`,
     `carve_asteroids`.
   - `probe / structure`: the section and damage screenshots and loops, the
     integrity ranges, every `stress_` range, `bug_carve_apply`,
     `shape_bench`.
   - `probe / world`: the flight, gravity, orbit and NOVA OS screenshots,
     the flight and helm loops, attitude, turn limit, thrust, scenario
     grammar, cinematic, player path, outcomes, and the world benches.
   Six directory-plus-prefix shards (`screenshot_`, `loop_`, `system_`,
   `stress_`+`bug_`, playable) is the mechanical fallback if subject
   judgement is unwanted, but it leaves `screenshot_editor` as the long
   pole.
2. Rewrite the "Probe matrix covers every example category" guard as a
   union check: expand each shard's list and fail when the union is not
   the whole `[[example]]` catalog or when an example appears twice.
3. Keep `fail-fast: false`. The probe summary step and the artifact names
   carry the shard name.
4. Re-measure after the split and record the new table here. The estimate
   from run 34317217860's per-example times is 15-17 minutes of runs per
   shard plus the six-minute build, against 44 today.

## Flaky, hold down while here

`crates/nova_probe/src/capabilities/timeline.rs`
`a_second_recorder_on_one_path_is_refused_not_torn` failed once in the check
job (run 34313476074, 2026-09-09 05:25) at the re-arm after `drop(first)`
and passed on the next push with no change to the file. Find the race (the
lock outliving the dropped App, or the path reused) and pin it.

## Not in scope

Cheapening the release `dist` profile or the check job's test split; those
change the shipped binary or the test contract.
