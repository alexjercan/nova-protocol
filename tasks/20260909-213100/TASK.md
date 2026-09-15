# Split the probe matrix into balanced shards

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.14.0, ci, probe, tooling

## Goal

Shorter CI on every push, so the review-heavy cycle gets its verdicts faster
and adding systems ranges does not lengthen the critical path. Owner
(2026-09-09): "we can do more splits to have easier CI, so we parallelize
more."

## Measured (run 34521413852, 2026-09-10 v0.13.2, warm cache, all green)

The last fully green run. Its three `probe-runs-*` artifacts carry a
`duration_secs` per example, which is what the split is sized on.

| job | wall | runs | fixed | examples |
|-|-|-|-|-|
| probe / screenshots | 54.6 min | 46.4 min | 5.6 min | 44 |
| probe / systems | 35.2 min | 27.4 min | 5.5 min | 40 |
| probe / playable | 21.2 min | 14.0 min | 6.3 min | 15 |
| fmt / clippy / test | 21.1 min | - | - | - |

A shard's wall is `fixed + sum(runs) + ~3.5 s per example`, where fixed
(checkout, apt, cache restore, linking) measured 338, 332 and 375 s. So the
runs are the only movable part, `check` at 21 min is the floor worth aiming
at, and screenshots alone was 2.6x that.

Per-example times were re-read from run 34853733506 (2026-09-14, 111
examples, some red) where it had run something the green run had not; a
FAILED example's clock is its hang, not its cost, so the green value wins
wherever there is one. Eleven ranges landed after 2026-09-14 and have never
run in CI: `system_ai_patrol`, `system_flight_legs`, `system_gravity_wells`,
`system_helm_orders`, `system_hud_scales`, `system_mission_hud`,
`system_railgun_hulls`, `system_settings_persist`, `system_ship_audio`,
`system_torpedo_capital`, `system_wreck_lock`. Each is carried at 40 s, a
little above the measured median `system_` range. `system_collision_damage`
is carried at its 289 s FAIL clock, which is an over-estimate, not an under.

## Landed 2026-09-15

Nine SUBJECT shards, 121 examples, no directory left in the matrix. Sized
off the table above; the estimate is 15.8-21.1 min per shard against 54.6
today, so the whole run becomes `check`-bound.

| shard | examples | runs | estimated wall |
|-|-|-|-|
| editor | 7 | 705 s | 18.0 min |
| menu | 17 | 694 s | 18.4 min |
| hud | 14 | 550 s | 15.8 min |
| gunnery | 16 | 828 s | 20.6 min |
| ordnance | 11 | 799 s | 19.8 min |
| hull | 15 | 711 s | 18.6 min |
| destruction | 8 | 753 s | 18.9 min |
| flight | 20 | 848 s | 21.1 min |
| worldgen | 13 | 867 s | 21.0 min |

- `.github/workflows/ci.yaml`: `matrix.include` carries `shard` plus a
  whitespace-separated `examples` list; the step squeezes that into the comma
  spec `resolve_spec` already takes. Title, summary and artifact name follow
  `matrix.shard`. `fail-fast: false` and the shared `probe-debug` cache stay.
- `scripts/check-probe-shards.py` replaces the old category-vs-directory
  guard: it expands the matrix out of the workflow and fails unless the
  shards partition the `Cargo.toml` catalog exactly. `catalog_drift.rs` owns
  disk <-> `Cargo.toml`, so the pair means every example file runs once.
- `scripts/probe-summary.py` drops the `spec` caption once the spec is a
  list - the row count already says it, and the names were 400 characters.

Still open: re-measure on the first green run after this lands and replace
the estimate column with what the shards actually cost. Watch `editor`
(both editor outliers) and `destruction` (the `system_collision_damage`
guess).

## Flaky, hold down while here

`crates/nova_probe/src/capabilities/timeline.rs`
`a_second_recorder_on_one_path_is_refused_not_torn` failed once in the check
job (run 34313476074, 2026-09-09 05:25) at the re-arm after `drop(first)`
and passed on the next push with no change to the file. Find the race (the
lock outliving the dropped App, or the path reused) and pin it.

## Not in scope

Cheapening the release `dist` profile or the check job's test split; those
change the shipped binary or the test contract.

## Ranges waiting on this split

`20260909-214629` closed on 2026-09-14 with one item it could not run: its
seven ranges had no shard of their own, so they were run directly three times
each instead. All seven are in `probe / menu` now and must come back green
there three runs in a row:

`system_session_loop`, `bug_failed_assets`, `bug_refused_scenario`,
`bug_outcome_pause`, `bug_menu_fallback`, `system_command_shell`,
`system_pause_settings`.

`20260909-213441` adds `system_scenario_picker` (renamed from
`bug_menu_picker`) and `system_settings_persist`, both also in `menu`.
