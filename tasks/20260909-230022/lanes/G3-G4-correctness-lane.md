# G3+G4 correctness lane

agent: agent-a318df9d833c645b0
last_ts: 2026-09-09T20:50:09.013Z
stop_reason: end_turn
records: 340

## Dispatch prompt

```
You are the Correctness lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/correctness.md

Range under review: `45372be15^..ba0cc418f` plus two later standalone commits, `62043f3b5` and `9e69ac196`. Four commits in all:
- 45372be15 "Keep the status bar out of every capture that keeps the HUD"
- ba0cc418f "Tell a driver the truth about the radar dwell and the aim cap"
- 62043f3b5 "Let the scenario sweep tolerate a root that already died"
- 9e69ac196 "Compile the news loop producers without the debug feature"

Bundle, already built. Do NOT re-derive the range:
- /tmp/nightly-20260909/g34/log.txt
- /tmp/nightly-20260909/g34/stat.txt
- /tmp/nightly-20260909/g34/diff.txt   (the last two commits appended under `=== <sha> ===` headers)

Touched crates: nova_bench (gesture, manual, observation), nova_channel (apply, lib), nova_debug (harness), nova_probe (capabilities/snapshot), nova_scenario (loader/lifecycle), plus several examples and docs/environment-variables.md.

Focus on: the scenario sweep's new tolerance for an already-despawned root (does it now swallow a real failure?), the nova_channel apply path, the probe snapshot capability, and whether the `#[cfg(feature = "debug")]` gating added by 9e69ac196 is on the right items and does not leave dead code or a broken build under either feature set.

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do not run a rendered example, a probe, or any benchmark.
- Never run the workspace test suite or Clippy. `nix develop --command cargo check -p <crate>` or `cargo test -p <crate> --lib <filter>` is allowed when decisive.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

## Findings

**MAJOR - `crates/nova_channel/src/apply.rs:262` - a `section` stop is REFUSED when the mount is gone, so the source the press pushed is never lifted.**

`apply_section` resolves `section_source(world, id)` fresh on every line and returns `refuse` when it resolves to nothing. On a *release* that leaves the synthesized press down with nothing able to lift it.

Failure scenario: driver sends `input section.pdc_forward_port start` with Flight live → `dispatch::press_source` presses `KeyU` in `ButtonInput<KeyCode>`. The mount is destroyed before the driver's next turn (`integrity::explode::despawn_destroyed_that_does_not_detach`, `crates/nova_gameplay/src/integrity/explode.rs:194`, `try_despawn`s the section entity; the detach path does the same for an explodable one). The driver sends `input section.pdc_forward_port stop` → `section_source` returns `None` → `refuse("no section \`pdc_forward_port\` on the ship")` → `dispatch::press_source` is never called and `KeyU` stays pressed for the rest of the process. Nothing on the wire says so: `inputs.held` is the referee's own memory and `gesture::expand` (`crates/nova_bench/src/gesture.rs:245`) does `held.remove(wire)` at expansion time, before the game sees the line, so the view reports the trigger up while the world has it down. Any other consumer authored onto that source - a second mount sharing a key in `input_mapping`, or a named action bound to it - then reads a held button for the rest of the run.

This is the same invariant the commit's new test names ("the stop must reach the bound source, or the mount is left firing"); the test only covers the lowered-context half. The lane directly above it does not have the hole: `apply_input` releases through `dispatch::apply` → `held_source`, which resolves what the *press* pushed and falls back to the binding (`crates/nova_input/src/dispatch.rs:166`).

Actionable: record the section's source in `DrivenPresses` (or a section-local twin) at press time and let a Release let up the recorded source unconditionally, refusing only an unresolvable *Press*. Add the sibling test - press, despawn the section entity, release, assert the source is up.

Not a BLOCKER: it predates this range (the range adds only a test here), and it needs a mount to die between two of the driver's lines.

**MINOR - `crates/nova_scenario/src/loader/lifecycle.rs:115` - the `try_despawn` fix ships with no test and no range that stages the case it fixes.**

The gate the commit message names is real - `crates/nova_probe_cli/src/evaluation/checks/log_clean.rs:9-20` fails a run whose log carries ``Encountered an error in command `bevy_ecs::system::commands::despawn`: The entity ... does not exist``. But nothing in the tree stages "a scenario ends on the same frame its ship dies". `lifecycle.rs` has ~1000 lines of inline tests and none queues a despawn of a scoped root before running the teardown; `system_outcomes` is the closest range and walks Die → Defeat overlay → Retry, which puts the teardown many frames after the death (and a shown outcome pauses virtual time, per its own module doc). So the regression can come back and `log_clean` will have nothing to catch it on.

Actionable: either add a `lifecycle.rs` test that despawns a scoped root through `Commands` and then runs `teardown_scenario_entities` over a query that still lists it (it will not fail without the fix - bare `despawn` only warns - but it pins the tolerance), or name the range that reproduces the same-frame case and say so beside the comment.

Not higher: the fix itself is right, matches the established pattern (`explode.rs`, `lifetime.rs`, `asteroid_carve.rs`, `nova_os_ui`), and cannot swallow anything but "entity already gone" - `q_scoped.iter()` yields each scoped entity once, so the only double-despawn is a scoped descendant of a scoped ancestor or an external despawn, and Bevy's generation bump means a stale `Entity` can never hit a reused slot.

## Checked

- **Scenario sweep tolerance (62043f3b5).** Read `teardown_scenario_entities` whole. `try_despawn` swallows only "entity does not exist"; every other command failure still reports. Both call paths (`unload_scenario`, and the load path's tear-down-then-spawn) queue despawns before spawns in one `Commands` queue, so no ordering hazard. Confirmed `log_clean` is the gate that was failing.
- **`nova_channel` apply path.** Read `apply_input`, `apply_section`, `section_source`, `dispatch::press_source`/`apply`/`held_source`. Ran the new test: `cargo test -p nova_channel --lib section_stop` → 1 passed. The press-only gate the test claims is present and the test's fixture matches what production spawns (`PlayerSpaceshipMarker` + `SpaceshipRootMarker` root, `SectionMarker` + `EntityId` + `SpaceshipTurretInputBinding` child).
- **Probe snapshot capability.** `radar_record`'s gate is `dwell_target.is_some() && dwell_needed > 0.0`, which is right: `is_dwelling()` also requires `dwell_secs < dwell_needed` and would go false on the exact frame a holding driver waits for. Ran `cargo test -p nova_probe --lib dwell_fill` → 1 passed. Verified the `SNAPSHOT_SCHEMA` doc correction: `git show v0.12.0:...snapshot.rs` has `SNAPSHOT_SCHEMA = 1` and no radar block at all, and `dwell_fill` first landed in `ab60b2998` (`v0.13.0~38`), so "that field never shipped" is accurate.
- **`#[cfg(feature = "debug")]` gating (9e69ac196).** Traced every newly gated item to its only consumers (`DriftClock`/`DRIFT_*`/`LOOP_SECS`, `KILLED_CELL`/`EYE`/`LOOK`/`*_SECS`, `ARRIVAL_MARGIN`/`TRACE_ENV`, `LEG_END`/`CROSSER_SPEED`/`SEND_AT_X`/`ORDER_KEY`, `EYE`/`LOOK`), all inside gated fns. `LOOP_AFTERMATH_SECS` is correctly *un*gated: `close_the_loop` (`stress_hull_collapse.rs:537`) is ungated and reads it. Built both feature sets: `cargo check --example {6 touched}` under default features and under `--features debug` - both clean, no `dead_code`/`unused` warnings. Also ran `cargo check --examples --keep-going` under default features: no errors, so the commit is complete, not partial.
- **`nova_bench`.** `MAX_AIM_TICKS = 60 * TICKS_PER_SECOND` = 3600, unchanged in value; both are `u64`. `manual.md` says "1 to 3600" and the parser's own refusal message uses the same phrasing. Ran `cargo test -p nova_bench --lib -- aim_tick_cap lost_its_hull` → 2 passed. Confirmed the `owner: null` fixture matches production: `ordnance_record` writes `owner` through `label_of`, which yields `Value::Null` for a despawned owner entity.
- **`targeting.md` dwell claim.** `RadarState` is inserted on `Start<RadarHoldInput>` (press), not at the hold threshold, and `radar.candidate` is written *before* the `if !hold_fired { continue; }` guard - so `candidate` really is populated during the search window while `dwell_fill` is null. `RADAR_TAP_SECS = 0.25` against the channel's `TICK_DT = 16_667 µs` gives the stated ~15 ticks.
- **`harness.rs` deadline doc.** Verified against `LoopCapturePlugin::build` (`nova_autopilot/src/loops.rs:51-72`): the `ManualDuration` pin happens at plugin build, behind `capturing()`, at `profile.frame_duration()`, and Bevy drives `Time<Real>` from that strategy. `shoot` returns early unarmed (`harness.rs:582`) and `shot_written`/`loop_written` return a constant-true predicate unarmed (`predicate.rs:197,215`). Default 30 fps → 1/30 s frame step, well under `Time<Virtual>`'s 0.25 s `max_delta`; no example overrides `fps`.
- **`stress_torpedoes` deadline doc.** 45+15+90+90 = 240. CI sets `NOVA_AUTOPILOT_DEADLINE: 280` (`.github/workflows/ci.yaml:200`). `DEFAULT_DEADLINE_SECS = 120.0`. `clean_pass_env` does not set the variable and `fps_window_and_deadline_env()` is only extended onto the `NativePass::FrameTime` env (`run.rs:337`), so `--correctness-only` really does inherit the flat 120 s.
- **`environment-variables.md` railgun claims.** `aftermath_window()` panics on a non-number and asserts `> 0.0`; `live_cut()` panics on anything but `0`/`1` and is `false` unset; `capture-web-media.sh` does `rm -f "$file"` before the run (line 190) and `[[ -s "$file" ]] || exit 1` after (line 208). All accurate, including the "overwrote the staged slowed row" consequence.
- **`nova_channel/lib.rs` doc re-point.** `docs/keeping-docs-in-sync.md:83` routes the crate to `architecture.md` and `commands.md` and explicitly names no second wire reference; `docs/agent-bench.md` documents the *bench* and itself defers to `architecture.md` for `nova_channel`. Consistent.
- **`hide_status_bar` coverage (45372be15).** Surveyed every example: each producer either calls `hide_hud` (which drops the Status tier with the rest of the HUD) or `hud_instrument` (which now drops the bar), plus `system_lock_line_of_sight` which calls `hide_status_bar` directly. No producer keeps the HUD without dropping the bar, so the commit title holds. `loop_cockpit`/`loop_command_shell` are behaviour-identical after the de-duplication.
- **`wfc_arena` counter rescoping.** `Strike::shots` is zeroed on entry to "the lance fires" and `hits` on entry to both "both sides open up" and "the second salvo" - which is what the two consecutive `Strike::hit` beats need. `stage_the_strike` still installs both at 0 and still calls `disarm_the_rival_lances`.
- **`kit.rs` dedup.** `section_entity` is byte-for-byte the logic that was inlined in `section_health`.
- **Range roster.** No commit adds or removes an assertion in `examples/systems/`, so no `outcome:` slug or `catalog_drift.rs` roster change is owed.

## Not checked

- No rendered example, probe run, loop capture or benchmark - I do not hold the measurement slot. Every claim about what a walk *looks like* on screen (bar out of shot, loop framing, the railgun cuts) is unverified; I checked only the code path that would produce it.
- No workspace test suite and no Clippy, per the contract. I ran only three targeted `--lib` filters plus `cargo check` on the touched examples.
- I did not exercise the `nova_bench` referee end to end, so the `inputs.held`-vs-world divergence in the MAJOR is argued from reading `expand`/`condense`/`Referee`, not from a run.
- `bevy_enhanced_input`'s exact `Hold` edge tick is not pinned; "the first 15 ticks" in `pages/targeting.md` could be off by one at tick 15 (0.250005 s vs a 0.25 s threshold). Not reported - I could not ground which side of the boundary BEI lands on without running it, and a driver reading `dwell_fill` is unaffected either way.
- `docs/agent-bench.md:99` still describes `{"aim": ..., "ticks": K}` with no cap while `manual.md` now states 1..3600. Not reported as a finding - the manual is the agent's contract and agent-bench.md is incomplete rather than wrong - but it is the docs lane's call.
- Rustdoc was not built, so intra-doc links from ungated docs into newly gated items (e.g. `stress_hull_collapse.rs:55` → `LOOP_AFTERMATH_SECS`) were checked by reading only, not by `cargo doc`.
