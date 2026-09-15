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

Measured on run 34971013563 (2026-09-15), the first fully green run.
`fixed` is the wall the runs do not explain: checkout, apt, cache restore,
linking.

| shard | examples | runs | estimated | wall | fixed |
|-|-|-|-|-|-|
| gunnery | 16 | 1229 s | 20.6 min | 27.6 min | 369 s |
| ordnance | 11 | 943 s | 19.8 min | 23.4 min | 422 s |
| menu | 17 | 850 s | 18.4 min | 21.6 min | 390 s |
| flight | 20 | 805 s | 21.1 min | 21.2 min | 398 s |
| hull | 15 | 778 s | 18.6 min | 20.4 min | 396 s |
| worldgen | 13 | 676 s | 21.0 min | 17.6 min | 338 s |
| editor | 7 | 603 s | 18.0 min | 16.4 min | 356 s |
| hud | 14 | 528 s | 15.8 min | 15.2 min | 333 s |
| destruction | 8 | 509 s | 18.9 min | 14.7 min | 345 s |

The split holds: 14.7 to 27.6 min against the 54.6 min screenshots used to
cost, and `check` ran 22.9 min on the same run, so the sweep is now
`check`-bound everywhere but `gunnery`.

Read one shard's wall against the estimate with care - the runners vary
more than the split does. Between runs 34967468116 and 34971013563, at the
same commit for every range but one, `hud` moved 21.3 -> 15.2 min and
`ordnance` 23.4 -> 23.4 while its own examples each grew by a third.
`gunnery` is the one real change: `system_turret_gunnery` used to die in
its own stall at 208 s and now runs to the end in 361 s, which is the whole
of that shard's 9.7 min.

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

`editor` carries both editor outliers and came in among the cheapest
shards, and `destruction` cost less than the `system_collision_damage`
guess said it would even with that range running to the end (231 s), so
neither needs a reshuffle. `gunnery` is the one shard worth splitting again
if the sweep has to come down further: it is 4.7 min over `check`, and its
own longest range is 361 s of that.

## Red on the first sharded run (34959859343)

Seven of the nine shards came back green. The two that did not were both
found by the split rather than caused by it - neither range had ever
reached its own verdict in a directory job.

- `system_headless_crt` (`hud`): the pick named its target off a picture
  the reframe was still sliding, then aimed at where the settled picture
  shows that blip - inside a cluster, under a neighbour's label pill. The
  pick now waits for eight frames of the same layout (`e3699ac3e`).
- `system_collision_damage` (`destruction`): healthy but long. The sweep
  pinned `NOVA_AUTOPILOT_DEADLINE` at 280 s under a 300 s `--timeout`, so
  the timeout dial moved nothing, and two carriers coming alongside cost 24
  real seconds per simulated one on a two-core runner. The pin is gone and
  `--timeout` is 510, which sizes the harness deadline to 480 (`168fe4b64`).
  The range now finishes in 231 s.

Two more came out of the runs after that:

- `nova_editor`'s probe tests (`check`): `sync_editor_probe` reads the
  camera framing so it can report the step the inspector scrubs distances
  by, and neither test rig had that resource - ten cases failed validation
  the first time the workspace suite got past the clippy error in front of
  it. Both rigs now frame their eye 200 m out (`adc67f61b`).
- `system_turret_gunnery` (`gunnery`): five per-beat deadlines at their
  limit at once, one of them past it, all five sized on a desk box against
  a runner frame that costs about six seconds. The beats that wait on the
  world now share one bound at about three times the worst measured beat
  (`70ac2704b`).

Run 34971013563 is green end to end.

## Flaky, pinned 2026-09-15

`crates/nova_probe/src/capabilities/timeline.rs`
`a_second_recorder_on_one_path_is_refused_not_torn` failed once in the check
job (run 34313476074, 2026-09-09 05:25) at the re-arm after `drop(first)`.

Cause: the flock lives on the open file DESCRIPTION, and a sibling thread
that shells out gives its child a copy of the whole descriptor table. Until
that child reaches `exec`, O_CLOEXEC has not fired and the description the
test just closed is still open in the child, so the re-arm gets
`WouldBlock`. Timeline's own `run_start` is the shell-out: `resolve_git_sha`
runs `git rev-parse` once per armed recorder, and three sibling tests in the
module arm one.

Reproduced on two cores (`taskset -c 0,1`, `--test-threads 4`): 5 failures
in 400 runs, and 0 in 400 with `NOVA_PROBE_SHA` set, which skips the fork.
The re-arm now retries for up to two seconds; 0 failures in 1200 runs under
the same pinning. Product is unaffected - a game process arms its recorder
in `Plugin::build`, before anything has forked.

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

Done: all nine were OK in `probe / menu` on four runs running -
34959859343, 34963931897, 34967468116 and 34971013563 - with no change to
any of them in between.
