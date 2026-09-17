# Verification

Every command was run from the repository root through `nix develop`.

## Content pipeline

| check | result |
|---|---|
| `cargo run content gen` | exit 0, 15 files written |
| `cargo run content lint` | 0 errors, 0 warnings, 0 findings, 14 scenarios balance-audited, 0 acked |

Generated output reviewed entry by entry in `GENERATED-DIFF.md`.

Both were run AGAIN after the last comment sweep, to catch a de-lored doc line
that had reached a generated name or description. Neither moved: the same four
RON files are modified against HEAD, the catalog still holds 16 sections, 8
ships and 4 styles, and lint still reads 0/0/0.

## Compilation

| check | result |
|---|---|
| `cargo check -p nova_authoring --all-targets` | clean |
| `cargo check -p nova_wfc --all-targets` | clean |
| `cargo check --examples --all-features --keep-going` | one failure, pre-existing (below) |
| `cargo fmt` over all eight touched crates | `--check` clean |

### Pre-existing example failure, NOT from this task

    examples/screenshots/screenshot_comms.rs:131:13: error[E0560]:
    struct `NarrativeCueActionConfig` has no field named `channel`

Neither `examples/screenshots/screenshot_comms.rs` nor
`crates/nova_scenario/src/actions/mission.rs` is touched by this task
(`git diff HEAD --quiet` on both is clean). The field was removed by
`4982d089c "Keep the mod format to the things a mod authors"`, which landed
before this task started. `screenshot_comms` is the only example that still
sets it. Left for its own fix.

## Tests

`cargo test -p nova_authoring`: 10 binaries, 141 tests, 0 failed.

| binary | tests |
|---|---|
| lib | 115 |
| balance_audit_gate | 2 |
| campaign_membership | 3 |
| content_lint_gate | 3 |
| content_report_gate | 2 |
| content_ron_parity | 2 |
| ledger_campaign | 10 |
| lesson_media | 3 |
| lesson_wiki_links | 1 |
| doc-tests | 0 |

The other crates this task edits, `--lib` each. `nova_console` and `nova_os`
carry the completion fixtures that named `block_raider`; `nova_editor` lost the
`DEBUG_ONLY_STYLES` filter with the style it hid; `nova_wfc` now imports the
two promoted section ids instead of spelling them.

| crate | tests |
|---|---|
| nova_editor | 511 passed, 0 failed |
| nova_console | 15 passed, 0 failed |
| nova_os | 58 passed, 0 failed |
| nova_wfc | 17 passed, 0 failed |

## Probe: systems ranges

`cargo run --features dev probe run <name> --correctness-only`, one at a time,
each with its own `--out`. Every range reached `Playing`, ran its own
panic-on-failure assertions to completion and exited clean.

| range | verdict | wall |
|---|---|---|
| stress_hull_collapse | OK 6/8 | 20s |
| system_ai_combat | OK 6/8 | 68s |
| system_ai_evade | OK 6/8 | 32s |
| system_ai_patrol | OK 6/8 | 62s |
| system_collision_damage | OK 6/8 | 27s |
| system_flight_legs | OK 6/8 | 102s |
| system_gravity_wells | OK 6/8 | 13s |
| system_helm_orders | OK 6/8 | 26s |
| system_hud_scales | OK 6/8 | 10s |
| system_hud_shell | OK 6/8 | 20s |
| system_hull_scaling | OK 6/8 | 13s |
| system_railgun_hulls | OK 6/8 | 24s |
| system_torpedo_capital | OK 6/8 | 21s |
| system_wreck_lock | OK 6/8 | 17s |

6/8 is the full score for a `--correctness-only` run: `capture_simulated` and
`fps_within_baseline` read `N/A - not claimed` because no frame-time pass ran.

The first pass of this sweep failed `system_ai_combat` and `system_ai_evade`
with a scenario start refusal. That was a real defect the migration exposed;
see the last section of `DECISIONS.md`. The table above is the sweep after the
fix.
## The menu duel, rendered

    DISPLAY=:99 NOVA_MENU_BACKDROP=menu_duel \
      RUST_LOG=info,nova_ship=debug,nova_scenario=info \
      timeout 240 cargo run --features dev

Exit 124 is the timeout killing a menu that never ends. No error and no panic
in 595 lines. The arc, from the log:

| time | line |
|---|---|
| 12:47:41 | `spawn_ship_skin: ship 1651v1 clad in 61 plate(s)` and `2944v0 clad in 54` - victor and rival |
| 12:48:07 | `aggregate_ship_health: root 2944v0 collapsed structurally` then `detect_neutralized: ... EntityId("duel_rival")` |
| 12:48:12 | `ignite_cold_torpedoes: torpedo 4822v3 lights its drive` - the inline Breaker is away |
| 12:48:26-27 | `update_turret_point_defense: mount ... -> Some(4822v3)` on SIX mounts |
| 12:48:28 | `torpedo_detonate_system: 4822v3 fuzed 2.41u out`, `sever_disconnected_structures: 1651v1 split into 3 bodies`, `destroyed 39 nodes`, `detect_neutralized: ... EntityId("duel_victor")` |
| 12:48:38 | two 93-plate hulls clad - the next backdrop is up |
| 12:51:09 | a 61-plate hull clad, then scripted torpedoes at it - the carousel has come round |

So the inline bay launches, six point-defense mounts track the round and fail
to stop it (the armored Breaker warhead), it kills the victor, and the backdrop
hands off and cycles.

Absent from the player catalog: `assets/base/sections/base.content.ron` has no
`heavy_torpedo_section` (see `GENERATED-DIFF.md`), and the duel's inline copy
carries `hide_in_editor: true`, so the section drawer cannot offer it.
## Documentation

| check | result |
|---|---|
| `mdbook build` | HTML book written, no error |
| `cd web && npm run ci` | format:check, lint, test, build - all pass (webpack compiled successfully in 15.8 s) |

`npm run test` includes `tests/widgets.test.ts`, which this task edited.

Neither output carries a removed id: `grep -rl` over `book/` and over
`web/dist/wiki` + `web/dist/create` finds no `block_raider`, `block_carrier`,
`block_warship`, `heavy_torpedo_section`, `siege_railgun_lance_section`,
`sections/standard` or `sections/ordnance`.

`docs/agent-bench.md` and `crates/nova_bench/scenarios/README.md` described the
arsenal player as a warship and its ram target as carrier-sized, and named the
hunt and arsenal hostiles raiders. All three sentences moved with the fixtures.
`mdbook build` was re-run after that edit: HTML book written, no error, and the
grep above still comes back empty.

## Probe: screenshot and playable examples

The same command, one example at a time, over every screenshot and playable
example that names migrated content or shares a producer with one.

| example | verdict | wall |
|---|---|---|
| first_shift_map | OK 6/8 | 9s |
| first_shift_ships | OK 6/8 | 7s |
| greeble_catalog | OK 6/8 | 17s |
| lesson_advanced_ai_flight | FAIL 6/8 | 54s |
| lesson_combat_battery | OK 6/8 | 19s |
| lesson_combat_field | OK 6/8 | 15s |
| lesson_combat_lance | OK 6/8 | 19s |
| lesson_combat_moves | FAIL 6/8 | 11s |
| lesson_combat_radar | OK 6/8 | 12s |
| lesson_combat_reach | FAIL 6/8 | 9s |
| lesson_combat_torpedo_types | OK 6/8 | 14s |
| lesson_combat_torpedoes | OK 6/8 | 11s |
| lesson_flight_aim | OK 6/8 | 10s |
| lesson_flight_basics | OK 6/8 | 14s |
| lesson_flight_limits | OK 6/8 | 14s |
| lesson_novaos | OK 6/8 | 14s |
| lesson_readouts | OK 6/8 | 15s |
| lesson_start_dock | OK 6/8 | 14s |
| lesson_start_scene | OK 6/8 | 11s |
| loop_goto_standoff | OK 6/8 | 20s |
| loop_helm_orders | OK 6/8 | 17s |
| loop_player_flight | OK 6/8 | 15s |
| loop_torpedo_blast | OK 6/8 | 16s |
| screenshot_combat_hud | OK 6/8 | 19s |
| screenshot_combat_lock | OK 6/8 | 18s |
| screenshot_combat_wide | OK 6/8 | 19s |
| screenshot_contextual_hud | OK 6/8 | 14s |
| screenshot_hud_shell | OK 6/8 | 13s |
| screenshot_hull_juice | OK 6/8 | 18s |
| screenshot_railgun | OK 6/8 | 28s |
| screenshot_torpedo_run | OK 6/8 | 15s |

28 of 31 pass. The three that fail are investigated in the next section.

### The three failures are PRE-EXISTING, proved against a clean HEAD

A `sprout` worktree at `fbee23138` - the commit this work branches from, with
none of it applied - ran the same three probes. All three fail there, with the
same message and the same numbers:

| example | at HEAD `fbee23138` | with this work |
|---|---|---|
| lesson_advanced_ai_flight | hostile 1382 m out | hostile 1346 m out |
| lesson_combat_moves | panic at `:482`, mounts housed | same panic, same line |
| lesson_combat_reach | `0 firing cell(s), 20 held` | `0 firing, 20 held` |

Two of the three also have a known cause, and it is the harness clock rather
than content. `sheet_written` returns `true` at once when `NOVA_CAPTURE` is
unset (`crates/nova_autopilot/src/predicate.rs:229`), so a sheet beat written
as `until(and(sheet_written(..), frames(N)))` ends after N FREE-RUNNING frames
on the smoke path instead of N frames at `LoopProfile::fps`. Twenty free frames
is about a third of a second, and a 90 degree step at the mount's 180 deg/s
needs half of one. Run the same two on the capture path and both exit 0:

    DISPLAY=:77 NOVA_CAPTURE_DIR=<dir> NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
      cargo run --example lesson_combat_reach --features debug
    -> exit 0, `barrel discipline: 13 firing cell(s), 7 held,
       cell by cell [0, 0, 12, 66, 55, 66, 58, 56, 65, 55, 0, 0, 0, 0, 0, 50,
       56, 58, 64, 56]`

`lesson_combat_moves` also exits 0 there. `lesson_advanced_ai_flight` fails on
BOTH paths at the same distance, so its stall is not a clock artifact - the AI
raider engages from where it is rather than closing. All three are outside this
task and are left as found.

Nothing in the failures reaches this work. The reach assert measures the
PLAYER's battery, and the player is `block_gunship` - a retained hull whose
generated body is byte-identical (see `GENERATED-DIFF.md`). The other two
example files are unmodified by this task. Their one migrated input is
`hollow`'s raider, and the fixture is a faithful copy of the removed catalog
entry: same cells, same specials, same plate, same style, the same `part`,
`turret` and `cell_part` helpers, and a `presentation()` that reproduces
HEAD's generated `block_raider` sound set field for field.

## The editor drawer

Read off the regenerated catalog rather than off a screenshot, because
`gallery::catalog` and the drawer both filter on one field:
`.filter(|(_, section)| !section.base.hide_in_editor)`
(`crates/nova_editor/src/gallery/catalog.rs:93`).

`assets/base/sections/base.content.ron` holds SIXTEEN sections and NOT ONE of
them carries `hide_in_editor`, so the drawer offers all sixteen:

    reinforced_hull_section   light_hull_section
    cargo_hull_section        tank_hull_section
    basic_thruster_section    vector_thruster_section
    capital_thruster_section  basic_controller_section
    pdc_kinetic_turret_section       pdc_pierce_turret_section
    pdc_twin_kinetic_turret_section  pdc_twin_pierce_turret_section
    torpedo_section           lance_torpedo_section
    railgun_lance_section     docking_port_section

Against HEAD that set loses exactly two and gains none:
`heavy_torpedo_section` and `siege_railgun_lance_section` - the Breaker bay and
the siege lance, the two the task names as unbalanced. Every player-buildable
section is still there.

Styles go from five to four: `industrial`, `armoured`, `civilian`, `salvage`.
`placeholder` is gone, which is why `DEBUG_ONLY_STYLES` and the test that
pinned it could go with it - there is no longer a style to hide.

Ships go from eighteen to eight: `block_cutter`, `block_hauler`,
`block_workship`, `block_frame_tender`, `block_frame_tender_damaged`,
`block_gunship`, `block_picket`, `block_wreck_plate`.

## Probe outcome registrations

`crates/nova_probe_cli/tests/catalog_drift.rs` needs no change.

- `catalog_matches_disk` walks `.rs` files DIRECTLY under each category dir.
  `examples/shared/` holds only `dev_fixtures/`, so the walk adds nothing and
  no `[[example]]` block is owed. The dir itself is a dir, so the stray-file
  panic does not fire either.
- No `outcome:` slug moved. The diff touches only the prose beside two of them
  (`system_collision_damage`, `system_hull_scaling`), where `block_carrier` and
  `block_skiff` became "carrier" and "skiff" now that neither is a catalog id.
  Both examples pass their probe.

## The loose bench fixtures

Three of the five fixtures changed: `arsenal` authors its player hull inline,
and `hunt` and `slingshot` fly a picket where they flew a raider. Each played
under the scripted agent, which needs no model:

    cargo run --features debug bench play \
      crates/nova_bench/scenarios/<name>.content.ron --agent baseline \
      --ticks 1800 --deadline 240

| fixture | outcome | ticks | ammo spent | bad lines | exit |
|---|---|---|---|---|---|
| arsenal | none (sandbox, 0/0) | 1800 | 2379 | 0 | 0 |
| hunt | victory, 1/1 | 331 | 2309 | 0 | 0 |
| slingshot | none (sandbox, 0/0) | 1800 | 8094 | 0 | 0 |

`arsenal` is the one that had to be watched: its player hull is now authored
inline rather than taken from the catalog, and the run spends 2379 rounds with
`sections_lost 0` and `cheated false`, so the inline railgun, bays and mounts
all arrived and all fired. `hunt` still scores ITSELF - the baseline killed the
picket and the scenario closed on its own outcome rather than on the tick
budget, which is the substitution's real test. `slingshot`'s pursuer is alive
at 2500 m at the end, which is the fixture behaving as written.
