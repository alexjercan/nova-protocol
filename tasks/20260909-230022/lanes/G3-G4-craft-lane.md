# G3+G4 craft lane

agent: agent-a3b5f1a4d8de4d093
last_ts: 2026-09-09T20:44:53.335Z
stop_reason: end_turn
records: 219

## Dispatch prompt

```
You are the Craft lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/craft.md

Range under review: `45372be15^..ba0cc418f` plus `62043f3b5` and `9e69ac196`. Four commits:
- 45372be15 "Keep the status bar out of every capture that keeps the HUD"
- ba0cc418f "Tell a driver the truth about the radar dwell and the aim cap"
- 62043f3b5 "Let the scenario sweep tolerate a root that already died"
- 9e69ac196 "Compile the news loop producers without the debug feature"

Bundle, already built. Do NOT re-derive the range:
- /tmp/nightly-20260909/g34/log.txt
- /tmp/nightly-20260909/g34/stat.txt
- /tmp/nightly-20260909/g34/diff.txt   (the last two commits appended under `=== <sha> ===` headers)

The capture-side change (a shared way to hide the debug status bar while keeping the HUD) touches many example files at once: judge whether the abstraction landed in the right place, whether examples/screenshots/shared/ is now the single home for it or whether copies remain, and whether the `#[cfg(feature = "debug")]` gating added later by 9e69ac196 is the shape the repo already uses for this (see the debug-gated example pattern in examples/ and docs/development.md).

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do not run a rendered example, a probe, or any benchmark.
- Never run the workspace test suite or Clippy. `nix develop --command cargo check` on a single crate or example is allowed.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

## Findings

**MINOR - `examples/screenshots/shared/ring.rs:319` - the status-bar fix landed as a third identical copy of `hud_instrument`, not in a single home.**

`45372be15` folded `hide_status_bar` into `hud_instrument` in three places: `shared/ring.rs:319`, `shared/hollow.rs:460`, and a private copy at `screenshot_radar_lock.rs:287`. All three bodies are byte-identical, and each carries the same six-line docstring, which is itself a fourth restatement of the rationale already written on `hide_status_bar` (`crates/nova_debug/src/harness.rs:729`). So `examples/screenshots/shared/` is not the single home - it is two of the three homes.

Failure scenario: the radar-lock copy is private, so `grep "pub fn hud_instrument"` returns two hits. Widen what an instrument framing means - a second persistent widget, or `hide_status_bar` gaining a parameter - and both `shared/` copies get edited while the private third silently keeps the old behavior, and `screenshot_radar_lock`'s shipped still goes out with the widget in it. The docstring's own claim, "Dropped here rather than at each call site, so a producer cannot ship the bar by forgetting a line", holds only inside whichever set module a producer happens to `#[path]`-include: `examples/systems/system_lock_line_of_sight.rs:188` records the shipped `news-0130-lock-occlusion` loop with the HUD up and still hand-calls `hide_status_bar`, because it cannot reach `ring::`.

Change: one `hud_instrument` in `crates/nova_debug/src/harness.rs` beside `hide_hud` (711) and `hide_status_bar` (729), exported from the harness prelude; delete all three copies and the two duplicate docstrings. That is the lowest crate every consumer already shares, and it is symmetric with `hide_hud`. `examples/screenshots/shared/kit.rs` is the wrong home despite being reachable from all three - its module doc states the kit keeps "only geometry" and enumerates "What it holds, and nothing else".

Not higher: every capture routed through one of the three copies is correct today; this is drift risk and duplicated prose, not a defect that ships.

**MINOR - `examples/systems/stress_hull_collapse.rs:529` - `hide_hud` then `hide_status_bar` is two ways to say one thing, and it contradicts `hide_hud`'s stated contract.**

`hide_hud` sets `HudVisibility::Cinematic`; `HudVisibility::shows()` (`crates/nova_hud/src/lib.rs:149`) is false at that level for every tier, and `status_bar_persists_through_the_nova_os_but_cinematic_still_clears_it` (`crates/nova_hud/src/lib.rs:1163`) pins that Cinematic clears `HudTier::Status` even mid-NOVA OS. `hide_hud`'s own docstring (`crates/nova_debug/src/harness.rs:705`) says it drops the HUD "so the fps/version bar is out of shot".

Failure scenario: a reader who trusts this pair concludes `hide_hud` does not cover the bar, and starts adding `hide_status_bar` after every `hide_hud` - at which point the fifteen loop producers that call `hide_hud` alone all read as bugs, and a reviewer spends a pass on fifteen non-problems.

Change: drop line 529.

Not higher: behavior is identical either way. The line predates the range (`7fdd25222`), but `45372be15` is the commit that draws the `hide_hud` / `hide_status_bar` line and `9e69ac196` edits this same file, so it is this range's to settle.

**MINOR - `examples/playable/wfc_arena.rs:2759` - a nine-line body comment that explains the previous revision rather than the code under it.**

AGENTS: comment inside a body only where the reason is not recoverable, and explain constraints, not history. Both counters are now re-zeroed on entry to the beat that reads them (`2903`, `2962`), each with its own reason stated there. What is left here is a comment whose conclusion is that these two zeroes do nothing - it ends "The zeroes here are the baseline the subject is installed with, not the fix", a note about a superseded revision. `Strike` derives `Default` (`2516`), so `hits: 0, shots: 0` at `2770-2771` are the defaults spelled out.

Failure scenario: the next editor of `stage_the_strike` reads nine lines arguing about counter scoping, then finds the scoping is not here, and has to go read both cue sites anyway to learn nothing new.

Change: `*world.resource_mut::<Strike>() = Strike { subject: Some(subject), ..default() };` with a one-line comment pointing at `cue_the_lances` / `cue_the_tubes`.

Not higher: prose and a struct literal; no behavior rides on it.

## Judgements the brief asked for, with no finding attached

- **`#[cfg(feature = "debug")]` gating in `9e69ac196` is the shape the repo already uses.** Per-item attributes are the dominant idiom in `examples/` (437 occurrences), `loop_belt_compare.rs` already gated `LOOP_NAME` exactly this way before the commit, and the statement-level `#[cfg]` on `app.init_resource::<DriftClock>()` (`loop_belt_compare.rs:83`) matches `playable/widget_zoo.rs:92`, `loop_cockpit.rs:44` and `systems/system_hud_indicators.rs:179`. The module-level `#![allow(dead_code, reason = ...)]` used in `shared/*.rs` is correctly not what a single-target producer reaches for. `stress_hull_collapse.rs`'s removal of a gate is right: `close_the_loop` is ungated and reads `LOOP_AFTERMATH_SECS`.
- **`62043f3b5` is the correct simple change.** `try_despawn` is the established API here (27 call sites), and the comment's cross-reference to `integrity::explode` is accurate (`crates/nova_gameplay/src/integrity/explode.rs:174`). No shim, no option, no caller changed.
- **`MAX_AIM_TICKS = 60 * TICKS_PER_SECOND`** imports through `crate::prelude` as the house rule requires, and both names are added to `nova_bench`'s prelude in the same commit. The `manual.rs` test is inline, named as a behavior statement, and reads the const through the prelude.
- **`kit.rs`'s `section_health` now delegating to `section_entity`** is the same-logic-in-two-places rule applied in the right direction.

**Checked:** the three bundle files; `AGENTS.md` and both lane briefs; every `hud_instrument` / `hide_status_bar` / `HudVisibility::On` site in `examples/` and `crates/`; the HUD tier and visibility contract in `crates/nova_hud/src/lib.rs` including the two status-bar tests; `crates/nova_debug/src/harness.rs` around `hide_hud` / `hide_status_bar` / `nova_screenshot`; all six files of `9e69ac196` read whole or in context, against the per-item `#[cfg]` census across `examples/`; `crates/nova_scenario/src/loader/lifecycle.rs` teardown and the `try_despawn` census; `wfc_arena.rs` `Strike`, its two counter systems and all three cue beats; the new tests in `nova_channel/src/apply.rs`, `nova_bench/src/manual.rs`, `nova_bench/src/observation.rs` and `nova_probe/src/capabilities/snapshot.rs` for placement, naming and prelude imports; the diff scanned for new `#[allow]`. Ran `nix develop --command cargo check -p nova-protocol --examples` under default features - clean, no dead-code warnings, so `9e69ac196`'s gating is complete for the example surface.

**Not checked:** the cargo check ran against a warm cache, and I did not force a rebuild, so a replayed-clean unit is possible; I did not run `--all-targets` or the workspace default-features check CI runs. No test suite, no Clippy, no rendered example, no probe, no benchmark - I do not hold the measurement slot, so none of the capture claims about frame cost, `Time<Real>` pinning or deadline sizing in the `harness.rs`, `screenshot_railgun.rs`, `loop_command_shell.rs`, `system_torpedo_launch.rs` or `stress_torpedoes.rs` docstrings were verified against a run; the perf lane owns those. Documentation content (`docs/environment-variables.md`, `nova_channel`'s module doc, `manual.md`, `pages/targeting.md`, `SNAPSHOT_SCHEMA`'s bump rationale) and the changelog were left to the docs lane. I noticed but did not pursue that `wiki-nova-os-terminal.png` ships `NOVA OS v0.12.0+a5da7efdb` and `FPS: 37` on the NOVA OS topbar - a different widget from `HudTier::Status`, untouched by `hide_status_bar` and out of this range.
