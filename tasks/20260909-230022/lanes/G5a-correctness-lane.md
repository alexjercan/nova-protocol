# G5a correctness lane

agent: agent-aa9f18eec4385841a
last_ts: 2026-09-09T20:47:33.546Z
stop_reason: end_turn
records: 266

## Dispatch prompt

```
You are the Correctness lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/correctness.md

Range under review: the three largest new loop producers added by commit `7fdd25222` ("Draft the v0.13.0 news post with its media and widgets"): `examples/screenshots/loop_hull_generate.rs` (760 new lines), `loop_helm_orders.rs` (395), `loop_goto_standoff.rs` (333).

Bundle, already built and PATH-SCOPED to those three files. Do NOT re-derive the range:
- /tmp/nightly-20260909/g5a/log.txt
- /tmp/nightly-20260909/g5a/stat.txt
- /tmp/nightly-20260909/g5a/diff.txt

These are autopilot-driven capture examples that produce the webm loops embedded in the v0.13.0 news post. Focus on: whether each loop's beats can hang or exit early; deadlines counted in Time<Real> versus sim time (this repo has been bitten by `.deadline()` counting real seconds while `elapsed()` counts sim time, so a max_delta clamp burns world time on a software rasteriser); assertions or gates that would let a wrong capture ship green; unwrap/expect on a query that can be empty; and seeded RNG determinism, since a loop that is not deterministic re-shoots differently every time.

Note that a later commit `9e69ac196` added `#[cfg(feature = "debug")]` to some items in these files because `cargo check --all-targets` under DEFAULT features failed on six targets. Check whether the gating at HEAD is now complete and correct for these three files.

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do NOT run these examples, a probe, or any benchmark.
- Never run the workspace test suite or Clippy. `nix develop --command cargo check --example <name>` is allowed and cheap.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

## Findings

**MAJOR - examples/screenshots/loop_goto_standoff.rs:259 - `both_parked()` reads every autopilot disengage as an arrival, so a leg that failed closes the loop green.**

The gate is `engaged == 0` over `Autopilot` on the two ship ids, and the doc above it states the contract as fact: "the computer disengages itself on arrival, so a hull with no autopilot is a parked one" (lines 256-257). `crates/nova_ship/src/flight/autopilot.rs` removes `Autopilot` on four paths and only one of them is the arrival at line 984: line 207 "lost its flight computer, disengaging", line 256 "has no live engines, disengaging", line 570 "GOTO target `{target:?}` is gone, disengaging".

Failure scenario: a content change drops the controller section from `BLOCK_WARSHIP_SHIP_ID`, or the beacon entity captured as `AutopilotAction::Goto { target: mark }` at line 250 is despawned. `autopilot_system` strips `Autopilot` from both hulls on its first tick, inside the 0.2 s window the preceding step already grants (line 462). The first polled frame of "fly both legs to the park" (line 464) sees `engaged == 0` and advances. `report_the_park_points` logs `'standoff_warship' parked 600 m from the mark's centre ... gap to the orb face 5xx m (margin 150 m)` with `info!` and no check, `loop_end` fires 1.5 s later, and a ~2-second webm of two motionless hulls 600 m out is encoded. `AppExit` is success.

Change: gate on the arrival fact - both hulls at rest with `centre - radius - BEACON_RADIUS` within a few meters of `ARRIVAL_MARGIN` - rather than on the absence of a component, or at minimum `warn!` in `report_the_park_points` when a leg's measured gap does not match the margin the file calls "the proof of the rule".

Not BLOCKER: the shipped `news-0130-goto-standoff.webm` is correct (11.23 s), so nothing wrong has shipped yet; this is the gate for the next re-shoot.

---

**MAJOR - examples/screenshots/loop_helm_orders.rs:52 - the loop's subject is an OS-seeded gunfight and the file pins no seed, so a re-shoot is different footage and can cross the frame cap.**

`crates/nova_gameplay/src/plugin.rs:83-86` takes `EntropyPlugin::<WyRand>::default()` (OS entropy) unless `NOVA_SEED` is set. Turret fire draws from it for muzzle spread (`crates/nova_ship/src/sections/turret_section/firing.rs:91`), so the time the gunship needs to take the skiff's computer is a random variable. `main` (lines 52-65) sets no seed, the module doc's capture recipe (lines 11-14) names only `NOVA_AUTOPILOT`/`NOVA_CAPTURE`, and `scripts/capture-web-media.sh:108` passes an empty env column.

Failure scenario: the shipped `news-0130-helm-orders.webm` is 13.066 s at 30 fps = 392 of `LOOP_FRAME_CAP`'s 600 frames (`crates/nova_autopilot/src/loops.rs:105`). About 330 of those are the fight, which nothing in the file bounds. A re-shoot whose spread rolls worse - the skiff survives ~60% longer, or the gunship flies one more attack run - crosses 600 frames while `LoopPhase::Recording`, and `loops.rs:456` error-exits with "loop `news-0130-helm-orders` exceeded the 600-frame cap". The producer fails on a die roll, and every successful re-shoot is different footage from the one on the site.

Change: pin the seed for this producer (an in-process `NOVA_SEED` equivalent, or the env column in `capture-web-media.sh`, the way `examples/systems/system_headless_replay.rs:51` insists on it), and size the fight against the 600-frame cap rather than against `FIGHT_DEADLINE_SECS`.

Honest context: no loop producer in the roster pins a seed. This one is called out because it is the first whose recorded subject is a fight-to-the-kill, and it is already at 65% of the cap.

---

**MAJOR - examples/screenshots/loop_helm_orders.rs:204, loop_goto_standoff.rs:214, loop_hull_generate.rs:666 - every deadline inside an open loop is larger than the frame cap, so no named-beat abort can fire while recording.**

`crates/nova_autopilot/src/loops.rs:348` pins `TimeUpdateStrategy::ManualDuration(1/fps)` on the armed path, which drives `Time<Real>` as well as the simulation - the harness says so at `crates/nova_debug/src/harness.rs:30-37` and tells producers to "budget a beat inside a capture in frames". A step's `deadline` is `step_real` (`autopilot.rs:583,625`), so on the armed path it is `deadline * 30` frames. The cap is 600.

- `loop_helm_orders.rs:380` and `:389` set `FIGHT_DEADLINE_SECS = 120` (3600 frames) on two steps that are both inside the open loop.
- `loop_goto_standoff.rs:466` sets `LEGS_DEADLINE_SECS = 120` (3600 frames) on "fly both legs to the park", inside the open loop. Its own doc at line 211-212 says "the frame cap bounds the recording, this bounds a leg that never parks" - but the cap always fires first, so this bounds nothing on the capture path.
- `loop_hull_generate.rs:666-706`: every in-loop beat carries `BEAT_DEADLINE_SECS` (600 frames) or `STEP_DEADLINE_SECS` (900 frames), and the six `hold` steps carry no deadline at all.

Failure scenario: the gunship never leaves `LEG_START` because the AI never picks up the installed `ShipHelmOrder`. Instead of "step `open the loop and give the order` stalled", which is the whole point of the named-abort machinery, the run dies at frame 600 with "loop `news-0130-helm-orders` exceeded the 600-frame cap - shorten the loop, do not raise the cap", pointing at the wrong thing.

Change: express in-loop deadlines as a fraction of `frame_cap / fps` (under 20 s at the default profile), so a stalled beat names itself before the recorder gives up.

Not BLOCKER: the run still fails loudly; only the diagnostic is lost.

---

**MINOR - examples/screenshots/loop_goto_standoff.rs:419 - the park log fabricates zeroes for reads that failed, and never checks the number it prints.**

`centre` falls back to `0.0` when `Position` is absent (line 421) and `radius` to `0.0` when `HullRadius` is absent (line 432). A hull whose components did not resolve logs `parked 0 m from the mark's centre, hull radius 0 m, gap to the orb face -40 m (margin 150 m)` - a printed number, not a report that the read failed. The module doc at lines 50-55 calls this log "the proof of the rule". A negative or zero gap is not distinguishable from a real one by anything but a human reading the log.

Change: skip the ship and `warn!` the missing component rather than substituting `0.0`, and compare the computed gap against `ARRIVAL_MARGIN` in the log line so a wrong park reads as wrong.

---

**MINOR - examples/screenshots/loop_hull_generate.rs:715 - the two 1080p stills are shot two frames after a 720p to 1080p window resize, with no stillness settle.**

`size_the_window_for_the_figures` resizes the primary window from the loop profile's 1280x720 to `CAPTURE_RESOLUTION`, and `the_window_is_figure_sized()` (line 399-403) advances on `and(window_size_is(w, h), frames(2))`. The next step's `on_enter` shoots immediately (line 721). `frames(2)` is the recipe `window_size_is` documents for *reading a laid-out box* (`crates/nova_autopilot/src/predicate.rs:140-142`), not for a rendered frame: 16 sibling producers in `examples/screenshots/` put `frames(SETTLE_FRAMES)` (30) before a shot, and `SETTLE_FRAMES`' own doc calls that "the stillness figure alone". A swapchain recreation plus an editor-rail reflow inside two frames on lavapipe is exactly the case the constant exists for.

Change: `and(window_size_is(w, h), frames(SETTLE_FRAMES))`, matching the fleet.

Not higher: the shipped `news-0130-hull-plan.png` and `news-0130-editor-save-as.png` are both 1920x1080 and the walk asserts the on-screen text it photographs, so a torn frame would be caught by eye.

---

**MINOR - examples/screenshots/loop_hull_generate.rs:94 - `ZONE_CYCLE` hard-codes a starting zone of `any` that comes from shipped content the walk never reads.**

The chip's initial label is the grammar's authored zone for that part: `crates/nova_editor/src/ui/mod.rs:1319` seeds each row from `priced.and_then(|part| part.zone)`, and `next_zone` (`ui/rail.rs:303`) runs `None -> Bow -> Amidships -> Stern -> Dorsal -> Ventral -> Flank -> None`. The six-entry `ZONE_CYCLE` is only correct while `pdc_kinetic_turret_section` carries no zone. It carries none today (`assets/base/grammars/base.content.ron:40-42`), but that file is generated from the Rust builders.

Failure scenario: a builder change gives that part `zone: Some(Bow)`. Press 1 yields `mid`, `the_zone_chip_reads(row, "bow")` never holds, and the run aborts at "cycle the turret's zone 1/6: release, and it reads `bow`" after 20 s - loud, but for a reason nothing in the file connects to the grammar.

Change: read the chip's current label on entry and drive the cycle relative to it, or state the coupling on `ZONED_ROW` so a content edit knows it owns this producer.

---

**MINOR - examples/screenshots/loop_hull_generate.rs:483 - the wheel beat computes its scroll once on entry with no per-frame retry.**

`scroll_the_rail_to`'s third beat sends one `scroll_pixels(rail.center().y - node.center().y)` in `on_enter` and then waits 20 s on `the_rail_shows(name)`. Every other aim beat in the file and in the shared kit re-drives its input each frame: `press_the_zone_chip` re-aims in `each` (line 545), and `AutopilotPlugin::click_named` holds the hover with `keep_hovering_named` "so the beat recovers from the reflow instead of holding a stale coordinate until the deadline" (`crates/nova_autopilot/src/autopilot.rs:246-250`).

Failure scenario: the second call at line 700 wheels the block back under the freshly grown Scene tree. The tree is still reflowing when the delta is measured, the single wheel event lands short or is clamped at the viewport's scroll extent, and the beat stalls the full 20 s and aborts - or, inside the open loop, is overtaken by the frame cap (see the MAJOR above).

Change: re-measure and re-scroll in an `each` hook, the way the aim beats do.

---

## Checked

- Both feature paths compile clean for the three files: `nix develop --command cargo check --example loop_goto_standoff --example loop_helm_orders --example loop_hull_generate` and the same with `--features debug`, both exit 0 with no diagnostics beyond the pre-existing `proc-macro-error2` future-incompat note. **The `#[cfg(feature = "debug")]` gating added by `9e69ac196` is complete and correct for these three files**; nothing is over-gated, and `loop_hull_generate` needed no change.
- Clock semantics end to end: `autopilot.rs:583-594` (`step_real` from `Time<Real>`, `step_elapsed` from `Time`), `loops.rs:348` (`ManualDuration` pins both), `harness.rs:24-47`. Every `deadline`/`elapsed` pairing in the three files traced against the open-loop window and the 600-frame cap.
- Every step's advance condition for a vacuous-true or hang: `both_parked`, `order_interrupted`, `order_resumed`, `gunship_past`, `the_hull_landed`, `the_row_is_ticked`, `the_field_reads`, `the_field_holds_the_caret`, `the_zone_chip_reads`, `the_pointer_is_over_the_zone_chip`, `the_rail_shows`, `the_node_reads`, `the_landing_camera_is_posed`, `the_window_is_figure_sized`.
- The `retype` key batch against `nova_ui`'s `text_field_keyboard` (`crates/nova_ui/src/widget/text_field.rs:298-360`): `Key::End`, `Backspace`, `Enter` are all handled, all queued messages are drained in one frame, and `SEED_CLEAR`/`NAME_CLEAR` are within the fields' bounds.
- The zone-chip cycle against `next_zone`/`zone_label` (`crates/nova_editor/src/ui/rail.rs:290-313`) and the shipped grammar.
- Determinism: `loop_hull_generate` is genuinely seeded (`collapse(..., seed.0, ...)` at `crates/nova_editor/src/generate.rs:192-200`, no global RNG in the path). `loop_helm_orders` is not. `loop_goto_standoff`'s legs are physics, deterministic under `ManualDuration`.
- Shipped artifact durations via `ffprobe`: 8.67 s / 13.07 s / 11.23 s (260 / 392 / 337 of 600 frames), and the two stills at 1920x1080.
- `unwrap`/`expect`/indexing in range: the only `expect` is `pose_for_the_landing` at line 383, a verbatim copy of the shared kit's `pose_editor_camera` (`examples/screenshots/shared/ui_walk.rs:78-88`). Same guard, same house pattern - not reported.
- Category obligations: `Cargo.toml`'s example-category contract (a `screenshots/` producer owes a graded walk, not an assert and no `outcome:` marker) and `examples/systems/README.md`.

## Not checked

- I did not run any of the three examples, a probe, or a benchmark - I do not hold the measurement slot. Every runtime claim above is derived from the code and from the committed artifacts, not observed.
- No workspace test suite and no Clippy.
- The webm and PNG contents themselves - whether the footage shows what the news post says it shows is the human judgement the category contract reserves.
- Whether the run-level completion deadline (`DEADLINE_ENV`) is above these scripts' summed step deadlines; the value comes from the launching harness, and I did not trace `probe run`'s per-target budget.
- The `once: false` on both producers' `OnStart` event, and whether a scenario re-fire can invalidate the `Entity` handle `send_both_to_the_mark` captures. I flagged the despawned-target disengage as one trigger for finding 1 without proving that specific path fires here.
- `nova_ui`'s `scroll_viewports` clamping behaviour, which would decide how likely the single-shot wheel in the last MINOR is to land short.
- Whether the probe's log check treats a `warn!` line as a failure, which would matter for `pose_camera`'s "no scenario camera present yet" during the load window.
