# NOTES - Refactor nova_* crates for structure and clarity

## Problem Statement

Nova Protocol reached ~155k LOC across 16 crates through fast feature work with
thin review. The cost is **orientation**: a reader cannot tell from the folder
tree what owns a behavior, so every change starts with a grep. Half the
workspace is one crate (`nova_gameplay`, 77,761 LOC / 169 files), and 43% of
that crate is `hud/`, which contains a 14.3k-line terminal/windowing runtime
that is not a HUD. The map that would orient a reader - `AGENTS.md` - is
measurably wrong.

Two secondary costs:

- **Stale narrative.** Docs cite task artifacts ("see this task's DECISION.md",
  "DECISION fork 4", bare task ids) and record history rather than constraints.
  This prose ages independently of the code and has already rotted.
- **Lying surface.** Advertised behavior that does not exist: `render: bool`,
  a `debug` feature that is always on, a dead feature flag, pub items nothing
  reads.

It is **not**:

- A performance task. No runtime metric is a success criterion.
- A test-coverage task. Tests are a separate task the owner will create.
- A what-comment purge. Measured: 83% of comments are why-comments, a strict
  purge yields ~440 lines of 155k. That premise was tested and rejected.
- A licence to move code. A move that does not reduce the cost of finding or
  reading something is out of scope.

## Success criteria

Confirmed with the owner. Ranked.

| # | Criterion | How it is measured |
| --- | --- | --- |
| 1 | Cold-read navigability | A `benchmark/` suite in this task folder: questions with expected answers, run before and after by out-of-context agents AND by the owner. Owner reviews the suite before the baseline runs. |
| 2 | Deletion count | Net lines removed across three targets (below). |
| 3 | Reduced coupling | Expected to fall out of 1 and 2. Not independently gated. |

Deletion targets, all three in scope:

- **Stale narrative and prose volume.** Task-artifact references, history,
  duplicated manuals (`nova_probe/src/lib.rs` duplicates the probe SKILL.md),
  multi-paragraph rationale essays. Rule: a comment must survive the next
  refactor. Includes the 91 identical `/// Glob-import surface: ...` boilerplate
  doc lines (confirmed: delete all 91).
- **Duplicated implementations.** Three scroll-viewport clamps, the
  list+details screen pattern triplicated in `nova_menu`, the two near-parallel
  orbit-camera/blip scenes in `nova_os_map` and `nova_os_ship` (1,456 lines),
  keybind chips twice.
- **Dead and lying surface.** 16 pub items referenced only by their own prelude
  re-export, the fake `render: bool`, `nova_info`'s dead `debug` feature, the
  `nova_debug` feature leak.

## Constraints (verbatim from the owner)

- "I don't want this to be just shuffling code around but still getting to an
  actually good result."
- "It's more about readability and being able to go through the code structure
  fast and being able to tell what each module/system does from the folder
  structure."
- "Code should be self documenting, keep docs only for public APIs (make clippy
  happy). But in code comments should be kept minimal and only for actually
  important things - comment why not what."
- Benchmarks: "a ./benchmark folder in the task folder with some markdown /
  json files with the questions and expected answers or expected behaviour and
  test it before and after; I can also be a factor here, so we use both the
  agent and a human ... but I also have to review them first ... then me and
  some out-of-context agents try to get an evaluation of them **before we change
  the codebase**."
- CONVENTIONS: build a Rust analogue of `~/personal/scufris/CONVENTIONS.md`,
  derived "by checking old untouched files before using agents or tools on the
  repo". Method confirmed: I extract candidate rules with before/after
  snippets, owner accepts or rejects each.
- Tests: "should be a separate task, I will see to it **do not create it**."

## Context

### Benchmark design

Full protocol: `benchmark/README.md`. Summary:

Six personas, each isolating one information channel - `blind` (source tree
with every `.md` deleted), `docs` (`.md` + wiki, no source), `tree` (a file
listing, names only), `rustdoc` (generated docs, `[source]` stripped), `owner`,
`modder` (4 wiki modding pages + `webmods/`). Plus `human`, an optional second
control.

**Isolation is a property of the image, not of the transcript.** Each persona
runs in its own Docker container holding only its channel; the repo is never
mounted, and `tasks/` is in no image. The soft `/tmp` guardrail and its
post-hoc transcript audit are gone - a persona cannot read what is not there.
Residual hole: the network stays up for the API, flagged not blocked.

The deltas carry the signal. `docs - blind` is how much prose compensates for
structure and should shrink. `tree` is the literal test of the owner's stated
goal. **A refactor that raises `docs` but not `blind`/`tree` is the failure
this benchmark exists to catch.** `modder` is a pass/fail regression guard, not
a delta - the modding surface does not change in this epic.

Three tiers, one paper each. Tier 1: 30 locate questions, scored on
`tool_calls` (primary, continuous - **counted from the transcript, not
self-reported**), graded correctness, wrong-path detours, and confidence.
Tier 2: three design tasks (new ship section type, new NOVA OS app, new
scenario action + event) as three separate runs, each producing a NOTES.md
graded by a grader agent on ownership / completeness / no-phantom-structure /
cost of arrival. Both graded tiers award **one number `0.00`-`1.00`** per
question and per dimension - no right / partial / wrong buckets; the keys fix
the values for the cases they call out and everything else is the fraction of
required parts answered. Tier 3, the modder task, has the only objective verdict in the
suite - the mod either passes `content -- lint` and loads, or it does not.

Papers are **generated** per persona from the answer key, so questions cannot
drift from their answers and no persona is shown a question it is not asked.
Results aggregate to `aggregate.json` + `.csv` and render to a self-contained
`report.html`, modelled on how `nova_probe` collects artifacts.

Baseline runs *after* the AGENTS.md correction so the delta is attributable to
structure. Owner ratifies the question set and answer key first; every expected
answer carries a `file:line` citation, since whoever drafts the key also
designed the refactor.

**Status 2026-08-07: built, never run.** Ratification, a code review and a
placement decision come first - see `plan/lane02.md`.

### Evidence base

Seven parallel audits, 2026-08-07, at HEAD `4a8b55aa` (clean tree), plus a
six-agent code review and a clippy audit the same day at the same commit.

The review is written up in `notes/09` through `notes/15`, deduplicated and
ranked in `notes/16-findings-master.md`, and split into landable lanes in
`notes/17-lanes.md`. Every `file:line` in `16` was re-opened against the tree
before being recorded; `16` also carries 17 corrections and withdrawals, kept
visible so a later reader does not re-derive a rejected claim.

### Measured comment reality (rejects the original premise)

| Metric | Value |
| --- | --- |
| `//` lines in `crates/` | 9,047 of 155,587 LOC |
| Sampled "restates the code" | 11% (8 of 70 blocks) |
| Sampled WHY / constraint / guard | 83% |
| Commented-out code | 0 |
| TODO/FIXME/HACK/XXX | 3 (two share one tracker id) |
| Crates with `#![warn(missing_docs)]` | 16 of 16 |
| `///` blocks | 5,734, averaging 3.03 lines, 64% multi-line |
| Strict why-not-what purge | ~440-670 lines (~0.3% of LOC) |

Worst-offender ranking is flat: nova_gameplay 5.7%, nova_os 5.4%, nova_probe
5.1%, nova_menu 2.2%. No outlier. The problem is volume and staleness, not
noise.

### nova_gameplay: the four seams

Verified acyclic. Cross-seam edge counts from `crate::` paths:

```
FLIGHT->CORE 30    HUD->CORE 26    HUD->FLIGHT 4    NOVAOS->CORE 3    NOVAOS->HUD 2
CORE->FLIGHT 6     CORE->HUD 1
```

Layer order: **CORE <- FLIGHT <- HUD <- NOVAOS**. The two back-edges are all of:

| Site | Nature | Resolution |
| --- | --- | --- |
| `plugin.rs:107,111,115` | composition root, depends on everything | lifts up into the assembly crate; not a real back-edge |
| `camera/framing.rs:200` | `crate::flight::is_forward_aligned`, pure math | move helper to `math` |
| `sections/controller_section.rs:301` | `.after(crate::flight::NovaFlightSystems)` | real scheduling edge; invert to `flight` declaring `.before`, or shared set |

Owner's decision: layer the four crates. No shared-types crate.

Seam contents: CORE (sections 9.9k, integrity 2.4k, damage, relations, mesh,
physics, transform, gravity, lifetime, math, asset_ref, cooldown, beacon,
audio 2.7k, settings) - FLIGHT (flight 5.9k, input 12.3k) - HUD (hud minus
nova_os*, ~19k) - NOVAOS (`hud/nova_os` 8.6k, `nova_os_map` 2.5k,
`nova_os_ship` 3.2k, `nova_os_pointer_rig`).

NOVA OS additionally has no owner today: logic in `nova_os`, UI in
`nova_gameplay/src/hud/nova_os*`, state and settings in `nova_menu`
(`NovaOsMonitorSettings` at `nova_menu/src/lib.rs:109`, pause hooks at
`lib.rs:185-190`, persistence at `settings_store.rs:86`).

### nova_probe: the design does not match the code

Probe is two programs. The in-game library (`src/lib.rs:82-168`) links into
examples; the host CLI (`src/bin/probe/native/`, 2,401 LOC) spawns the example
as a **child process** and reads its files back. Collection happens in the
child, arming via env vars (`native/env.rs:93-108`); evaluation and rendering
happen in the parent (`run_report/`). The IPC boundary is the filesystem.

Consequences for the owner's original sketch:

- A single `NovaProbePlugin` spanning collect -> evaluate -> report is
  impossible. The truncated-timeline check requires the writer process to be
  dead.
- `evaluate -> report` **already exists cleanly**: `RunArtifacts::load` ->
  `checks/*` (one roster table, each check `evaluate(&RunArtifacts) -> Check`)
  -> `html.rs`. This is the best code in the crate. Rename, do not rebuild.
- `nova_timeline` / `nova_invariants` / `nova_frametime` are free functions
  returning plugins (`recorder.rs:64`, `invariants.rs:70`, `capture.rs:105`),
  not crates. They share a declare+arm shape but their configs do not unify
  (`.out()`, `.strict()/.monotonic()`, `.drive()`), so a `trait Capability` can
  cover declare+arm only. Two blockers: the name collides with the existing
  `enum Capability` (`contract.rs:38`), and `invariants` is not a peer - it
  writes into the recorder's sink and orders `.before(record_variable_changes)`
  (`invariants.rs:143`).
- Two of six checks (`process_exit`, `log_clean`) have no in-app collector at
  all - the host collects them from the child's stdio.
- The wasm gates are mostly not IO. Of ~20, an IO abstraction removes ~5 (the
  recorder wasm stub `lib.rs:113-141`, the contract writer `contract.rs:137`,
  the frametime CSV write). The rest guard process spawning and CLI code.

Owner's decision: split into `nova_probe` (in-game collection library) and
`nova_probe_cli` (host harness), rename to `capabilities/` `evaluation/`
`report/`, AND evict `fixtures.rs` (scenario builders parked here to dodge the
`catalog_drift` scan, admitted at `fixtures.rs:9-15`), `profile_sandbox.rs`
(XDG/mod-cache knowledge belonging to `nova_assets`), and `bin/perf_web.rs`
(the sole reason probe depends on the whole game).

### nova_events is correctly used - AGENTS.md is what is wrong

`nova_events/src/lib.rs:1-9` states it is "the event vocabulary shared between
gameplay and the scenario engine... the game-event kinds a scenario reacts to".
Usage matches: nova_scenario 50 refs, nova_gameplay 10, and those 10 are exactly
the scenario-observable moments (`OnDestroyedEvent`, `OnNeutralizedEvent`,
`EntityId`, `EntityTypeName`). Intra-crate game logic correctly uses
observer-on-marker (46 files).

The AGENTS.md line "Cross-subsystem communication through `nova_events`, not
direct coupling" reads as a general architecture mandate and misleads every
agent that reads it. Reword, do not migrate.

### AGENTS.md is stale (fix first, step one)

- The `nova_modding` row is wrong on 3 of 4 items. Bundle merge is
  `nova_assets/src/merge.rs` (701 LOC), portal client is
  `nova_assets/src/portal/*` (1,866), downloads/cache is
  `nova_assets/src/mod_cache.rs` (1,178). Real `nova_modding`: one file, 439
  lines, asset loaders only. Its own crate doc contradicts AGENTS.md.
- The `nova_events` line, above.
- No indication that `nova_gameplay` is half the workspace.

### Structural bugs found (deletion target 3)

| Site | Defect |
| --- | --- |
| `nova_gameplay/src/plugin.rs:40` vs `:77,:85,:111` | `render: bool` documented as gating meshes/HUD/particles; forwarded to one plugin only. Hanabi, skybox, post and the whole HUD are unconditional. The headless mode does not exist. |
| `nova_debug/Cargo.toml:18` + root `Cargo.toml:224` | hard-forces `nova_gameplay/debug`, and root dev-depends unconditionally, so every `cargo test` builds gameplay with `debug` on regardless of flags. |
| `nova_info/Cargo.toml` | `debug = []`, zero cfg sites. Dead flag. |
| `nova_gameplay` | 16 pub items referenced only by their own prelude re-export. |
| `nova_gameplay/src/plugin.rs:80-101` | 13 leaf plugins added directly; no `NovaTransformPlugin`/`NovaLifetimePlugin`. 27 SystemSets exist, 6 are ordered. |
| `nova_ui` | two plugins plus `widget::register`, a first-caller-wins `WidgetObserversRegistered` resource standing in for a plugin. |

**Added 2026-08-07 by the review** - the same target, now with proof:

| Site | Defect |
| --- | --- |
| `nova_ui/src/tween.rs` | The whole `Tween` subsystem: 421 lines, 11 tests, **zero consumers workspace-wide**. `TweenPlugin` is registered (`nova_gameplay/src/hud/mod.rs:301`) and runs four empty queries every frame. |
| `nova_ui/src/status_bar.rs:133,153` | `StatusBarStore` declared and `init_resource`d, never read or written. |
| `nova_gameplay/src/sections/torpedo_section/bay.rs:112` | `Without<SectionInactiveMarker>` excludes nothing - the only writer of that marker is guarded by `With<SectionMarker>`, which the spawner lacks. Reads as a live-safety gate, does nothing. |
| `nova_gameplay/src/objectives.rs:123` | `rebuild_lines` can never run - `ObjectivesPanelMarker` exists only inside its own file. `ObjectivesPlugin`'s only system is a permanent no-op. |
| `nova_ui/src/widget/panel.rs:112` | `panel_head` takes a `UiSkin` and discards it as `_skin`, so the Hardware skin leaves a green CRT header band. The parameter makes every call site believe otherwise. |
| `nova_ui/src/status_bar.rs:238` | The entity the caller spawns is never parented and never rendered - the observer copies its data into a new child of the root. `nova_core/src/lib.rs:290,297` spawns two orphans. |
| `nova_gameplay/src/hud/nova_os_ship/mod.rs:166`, `nova_os_map/mod.rs:139` | `NovaOsShipSystems` / `NovaOsMapSystems` declared as `SystemSet`s and never passed to `configure_sets` - zero references outside their own file. No ordering edge at all. |

The last four are one pattern the review named **"code that lies about its
guard"**: the code states an intent the mechanism does not deliver. That is the
owner's third deletion target with concrete instances rather than a category
name, and it is a CONVENTIONS.md rule candidate - **an unused parameter or an
inert filter must be removed, never renamed to `_`**, because the signature is
what the caller believes.

### CI gaps (bound the safety of any refactor)

- `cargo clippy --workspace --all-targets --features debug` runs **without
  `-D warnings`**. Warnings never fail CI.
- No wasm job. `nova_assets/src/portal/*` wasm paths are compiled by nothing.
- No default-features job. `cfg(not(feature = "debug"))` branches are unbuilt.
- Nothing is `continue-on-error`; the probe `run --all` gate is real.

Consequence: unused-import and dead-code fallout from a refactor can land green.
This must be closed before the large moves, or the benchmark is measuring a tree
CI does not check.

**All three measured 2026-08-07 rather than assumed** (`notes/09`):

| Gap | Measured |
| --- | --- |
| clippy without `-D warnings` | **0 warnings** at the CI configuration. The gap is real; the implied cleanup pass **does not exist** |
| no default-features job | CONFIRMED, **11 warnings**, all dead code in `examples/` unreachable once `debug` is off. cfg-gate them (~20 min), then add the job |
| no wasm job | ~~"probably rotted"~~ **WRONG.** All 14 crates type-check clean, exit 0. 7 warnings, one cluster: `nova_probe/src/report.rs` is dead on wasm and wants a `cfg(not(target_arch = "wasm32"))` gate |

And a fourth, found by the review: **the probe gate that CI does trust is
itself blind in four ways, three failing open.** See idea 1b.

### Other confirmed findings, by crate

| Crate | Finding |
| --- | --- |
| nova_assets | 27.5k LOC holding base content (`sections.rs`, `scenario/**`, 5.5k), the modding stack, the portal client, and the `content` CLI - which drags `clap` in and needs `#[doc(hidden)] pub` escape hatches (`lib.rs:34-46`). Authoring toolchain is split across two crates with `nova_scenario/src/lint/`. |
| nova_scenario | `render_scale.rs` (523) is a Bevy render lever with no scenario vocabulary. `loader/trackers.rs` is not loading. Real coupling violation: `world.rs:138-144` and `actions/mission.rs:512,534,554` write `nova_gameplay` HUD state directly. |
| nova_menu | ~25 systems registered flat in `lib.rs:83-219` with no SystemSet. `setup_menu_ui` spans `menu_ui.rs:32-540`. |
| nova_ui | prelude effectively unused: 81 in-src deep-path imports vs 3 prelude imports. `widget/button.rs` (863) bundles two paint backends plus a builder. |
| nova_core | `AppBuilder` assembly split across `new()` and `build()`; `use_default_plugins` means four different things (`lib.rs:151-179`); `log_filter_str` hand-lists nine crates twice and omits five. |
| nova_probe | no prelude; 184 deep-path imports, worst in the workspace. |
| nova_info | 15 LOC, one const, three dependents. Merge candidate. |
| nova_modding | 439 LOC, one file, asset loaders only. Merge candidate. |

### Risk register for the refactor

1. `nova_gameplay` - 103 of its 169 files changed in the last 40 commits; 38 of
   the workspace's 54 lint suppressions hide there.
2. Freshly vendored bevy-common-systems code (10 commits, one review round, one
   rewind). `nova_ui/src/status_bar.rs` (365 lines), `camera/chase.rs` (242),
   `camera/wasd_controller.rs` (233) landed with **zero** tests. Nothing pins
   their behavior if moved.
3. `nova_events/src/engine.rs` - 570 vendored lines, 4 tests, and it is the
   scenario dispatch path.
4. ~~Seven `unreachable!()` match guards (`nova_scenario/src/lint/ship.rs:443,
   769,772`, `lint/scenario.rs:712`, `mesh/slice.rs:67`).~~ **Corrected
   2026-08-07 - overstated.** Four of the five named sit inside
   `#[cfg(test)] mod tests` (opened at `ship.rs:314` and `scenario.rs:529`) and
   are test assertion helpers. **Only `nova_gameplay/src/mesh/slice.rs:67` is
   production code**, and it keeps the original concern: refactoring the
   matched enum converts a compile-time exhaustiveness check into a runtime
   panic. Confirmed independently by two reviewers.
5. wasm-only and default-features paths, compiled by no CI job.

**Re-ranked 2026-08-07 against the review.** The full re-ranking with evidence
is in `notes/08-tests-ci-risk.md`. The headline changes:

- **A new #1: `nova_probe` is a blind CI gate.** Every other item on this list
  is a risk of a *silent break*; this one is the risk that a silent break is
  not caught. See idea 1b.
- **A new #4: `nova_editor`** - 5 defects in 2,378 LOC against 13 tests, the
  worst defect density in the tree. It was not on this register at all.
- Item 2 (vendored bevy-common-systems code) is **partly confirmed, partly
  inverted**: `status_bar.rs`, `camera/chase.rs` and `camera/wasd_controller.rs`
  do have 0 tests, and `status_bar.rs` is where the reviewer found three
  defects - the aim was good. But `tween.rs` has **11** tests, and its problem
  is the opposite: well-tested code with zero consumers.
- Item 3 (`nova_events/src/engine.rs`) is **confirmed and the defect found** -
  `engine.rs:170` maps a serialization failure to `data: None`, which
  `nova_scenario/src/filters.rs:71` reads as "does not match", so a scenario
  silently never advances.
- Item 5 is **downgraded**: both paths were measured (above), and neither had
  rotted. The residual risk is that neither is *tested*, which is real but
  smaller than stated.

What the review **cleared**, and is worth not re-deriving: there is **no
reachable `unwrap`/`expect`/indexing panic in non-test code** anywhere in the
audited scope (four independent confirmations), the simulation core - flight
guidance, the QP throttle balancer, the PD controller, gravity, integrity - is
sound, and the `nova_os` terminal's UTF-8 arithmetic has no reachable panic
(three confirmations).

## Ideas

**Re-ranked 2026-08-07 against the code review** (`notes/09` through `notes/15`).
The original six ideas all survive; four are new, and the ordering changed
because the review found that the thing verifying every other idea is itself
broken.

Full lane definitions - dependencies, verification, sizes, and which side of
the benchmark-baseline line each falls on - are in `notes/17-lanes.md`. The
ranked findings behind them are in `notes/16-findings-master.md`.

| Rank | Idea | Lane | Change |
| --- | --- | --- | --- |
| 1 | Fix the map, close the CI gaps | L0 | unchanged at #1, now cheaper than believed |
| 2 | **Unblind the probe gate** | L1 | **NEW - straight to #2** |
| 3 | Build and baseline the benchmark | L2 | was #2 |
| 4 | **Untrusted input, data loss and persistence** | L3 | **NEW** |
| 5 | **Reconciler discipline and terminal input** | L4 | **NEW** |
| 6 | Delete the dead and lying surface | L5 | was part of #5, now concrete |
| 7 | **nova_editor** | L6 | **NEW - the crate was not on the list at all** |
| 8 | `nova_ui::screen` extraction | L7 | was part of #5 |
| 9 | nova_probe restructure | L8 | was #3 |
| 10 | nova_gameplay four-way split | L9 | was #4 |
| 11 | nova_assets / nova_scenario cleanup | L10 | was #6 |
| 12 | **Perf and small correctness** | L11 | **NEW** |

### 1. Fix the map, then close the CI gaps

Correct AGENTS.md (crate table, nova_events wording, gameplay size). Add
`-D warnings` to CI clippy; add a default-features check job; add a wasm
`cargo check` job. Settle CONVENTIONS.md by extraction and owner ruling.

Cost: hours. Zero code risk. Everything downstream depends on it - agents work
from the corrected map, and the CI gaps otherwise let refactor fallout land
green. **Must precede the benchmark baseline.**

**Amended 2026-08-07 - cheaper than this said, and two items were added.**

- `-D warnings` needs **no cleanup pass first**: the tree produces **0
  warnings** at the CI clippy configuration today. Measured, not assumed.
- The default-features job needs 11 dead example items cfg-gated first (~20
  min); the wasm job needs `nova_probe/src/report.rs` gated first (1 line).
  Both are ordering constraints inside the lane, not blockers.
- **Added: convert the 37 `#[allow(clippy::type_complexity)]` to
  `#[expect(..., reason = "...")]`.** The codebase already uses `#[expect]`
  with a reason in 4 places, so this enforces an existing local convention.
  The payoff is that rustc's `unfulfilled_lint_expectations` then reports every
  **stale** suppression on the next clippy run, at zero analysis cost - two are
  already known stale by eye. It pairs with `-D warnings`: together they turn
  suppression rot and refactor fallout from things nobody audits into things
  CI reports.

### 1b. Unblind the probe gate (new, and it goes before everything else)

`nova_probe` is itself the CI gate, and the review found it **blind in four
ways, three of which fail OPEN** - a run can verdict OK when it should FAIL.
A green sweep after a large refactor currently means less than it appears to.

- One unparseable artifact destroys the entire report, discarding the clean
  pass's evidence (`run_report/artifacts.rs:44`), contradicting the code's own
  comment at `run.rs:211`.
- A panicking wasm app verdicts OK, because the loader deliberately refuses to
  read the file its panic is in (`artifacts.rs:65`).
- An errored run inherits the previous run's OK verdict, so **CI passes on a
  commit that was never probed** (`native/sweep.rs:187`).
- Stale sweep-cell logs present as this run's evidence (`native/run.rs:29`) -
  this one fails closed, but it trains people to distrust the gate.
- Plus an unordered same-schedule `AppExit` write/read
  (`recorder.rs:126` vs `nova_autopilot/src/completion.rs:152`) that will
  surface as an unreproducible CI failure during a large refactor, when
  everyone assumes the refactor caused it.

Cost: contained, behavior-only, needs its own fixture-driven verification
because the thing being fixed is the thing that normally verifies. **Every
other idea below is verified by this gate.** Fixing it is what makes the rest
of the epic checkable. Runs in parallel with idea 2's owner-review time.

### 2. Build and baseline the benchmark

`tasks/20260806-121625/benchmark/` - questions with expected answers, owner
reviews, then out-of-context agents and the owner both run it against the
current tree. Baseline is recorded before any code moves.

Cost: a day plus owner review time. This is the gate that makes 3-6 provable
rather than churn.

### 3. nova_probe -> nova_probe + nova_probe_cli

Split at the real process boundary. `capabilities/` `evaluation/` `report/`
inside each half as the names of the actual stages. Collection-side bundle
plugin preserving per-example config. Evict `fixtures.rs`, `profile_sandbox.rs`,
`bin/perf_web.rs`. Add a prelude (184 deep-path imports).

Cost: contained blast radius, no gameplay risk, and probe is the tooling the
rest of the epic is verified with - so it goes first among the code moves.
Caution: probe is itself the CI gate; a mistake here blinds the gate.

### 4. nova_gameplay four-way split, one seam at a time

Order: NOVAOS (leaf, 14.3k, biggest single navigability win, and it also gets
one owner - pulling `NovaOsMonitorSettings` and the pause hooks out of
nova_menu) -> HUD -> FLIGHT -> CORE. Resolve the three back-edge sites first.
`plugin.rs` lifts into the assembly crate.

Cost: the epic's bulk and its highest risk. Every seam needs the benchmark rerun
and a probe `run --all`. Items 2 and 5 of the risk register apply directly - the
vendored untested code is inside the seams being cut.

### 5. Deletion sweep

The three targets. Best done per-crate as each crate is otherwise touched,
rather than as one pass, so that stale prose is judged against the code it
now sits beside.

Cost: mechanical for narrative and dead surface; the duplicated-implementation
half is real refactoring with regression risk and should be its own task per
duplication site.

**Amended 2026-08-07 - the target now has names, and one scheduling rule.**

The review proved two entire features dead, which is the strongest evidence yet
for this target:

- **The whole `Tween` subsystem** - `nova_ui/src/tween.rs`, 421 lines, 11
  tests, `TweenPlugin` registered at `nova_gameplay/src/hud/mod.rs:301` and
  running four empty queries every frame. **Zero `Tween<T>` is spawned anywhere
  outside its own tests** (verified 2026-08-07 by grep across `crates/`,
  `src/`, `examples/`).
- **`StatusBarStore`** - declared at `nova_ui/src/status_bar.rs:133`,
  `init_resource`d at `:153`, never read or written. Those two lines are the
  only hits workspace-wide.

And "lying surface" now has four concrete instances rather than a category
name: `torpedo_section/bay.rs:112` (a `Without<>` filter that excludes
nothing), `objectives.rs:123` (a system that can never run),
`nova_ui/src/widget/panel.rs:112` (a `skin` parameter discarded as `_skin`),
`nova_ui/src/status_bar.rs:238` (an entity that is never rendered).

**Scheduling rule: this lands AFTER the benchmark baseline.** Deletion count is
success criterion #2, so lines deleted before the baseline is taken never enter
the ledger. This is the opposite of idea 1, and the two are easy to confuse
because both look like cheap doc-ish work.

### 5b. The behavior-only bug lanes (new)

The review found ~60 defects that exist today, independent of the restructure.
They are **behavior-only** - no file moved, no symbol renamed, no doc changed -
so they do not disturb the benchmark baseline and can run in parallel with
idea 2's owner-review time. Grouped by how they cluster:

| Lane | Contents |
| --- | --- |
| Untrusted input, data loss, persistence | Mod content arrives from a remote portal and from files the player may have edited, so a panic reachable from it is a defect. A corrupt index **silently deletes every other installed mod**; every persisted store is written non-atomically, which is what produces the corrupt index; `fire_rate: 0.0` panics the process on ship spawn; two DSL decoders and the dependency walker recurse unbounded; a serialization failure makes a scenario silently never advance |
| Reconciler discipline and terminal input | Ctrl+letter types into the NOVA OS prompt; the `f32::MAX` scroll sentinel is never cleared; four stale-`Local<T>` instances; five unguarded per-frame `DerefMut` writes forcing UI relayouts. **This must precede the NOVAOS seam move**, or the same defects get fixed after 14.3k lines have shifted |
| nova_editor | Five defects in 2,378 LOC against 13 tests - the worst defect density in the workspace, and the crate was not on this list at all. It is small enough to read whole in one sitting |
| Perf and small correctness | **Every fired bullet allocates a new Mesh and a new StandardMaterial** at 100 rounds/s per muzzle - almost certainly the largest single perf defect, and it sits directly under the probe's FPS baseline check. Plus the engine-spool duplicate below |

The single best cost/benefit item in the review: **extract the shared
engine-spool loop from `flight/autopilot.rs:877` and `flight/manual.rs:142`.**
One edit kills a 16-line byte-identical duplicate **and both copies of the
workspace's only real per-tick complexity bug** - an O(ships x thrusters x
thrusters_on_this_ship) linear scan, every FixedUpdate tick.

### 6. nova_assets / nova_scenario cleanup

Extract the authoring toolchain (`lint_walk`, `balance`, `content_report`,
`scenario_generation`, `bin/content`, plus `nova_scenario/src/lint`) into one
crate. Move base content out. A `Storage` trait mirroring the existing
`PortalTransport` pattern for the wasm KV gates. Route scenario -> HUD through
events. Lift `render_scale` out of nova_scenario.

Cost: independent of 4, so it can run in parallel. The `Storage` trait is the
one item that makes currently-untestable wasm paths testable natively.

**Amended 2026-08-07.** ~~The wasm paths have probably rotted.~~ **WITHDRAWN** -
all 14 crates type-check clean on `wasm32-unknown-unknown`, exit 0. The
`Storage` trait is still justified, by **testability and gate removal alone**.
Do not re-argue it from bit-rot.

And it now pairs with a defect: the atomic-write fix touches
`persist.rs:91`, `mod_cache.rs:521`, `portal/catalog.rs:197` and
`bin/content.rs:103` - **the same four files**. "Write atomically" belongs in
the trait as a contract rather than repeated as a convention at four call
sites, so the atomic-write fix should be the change that introduces the
trait's write method.

### Deliberately rejected

| Idea | Why it lost |
| --- | --- |
| Strict what-comment purge as the deletion metric | Measured at ~440 lines of 155k. Tested and rejected on evidence. |
| Full `nova_events` migration for gameplay's 46 observer sites | Based on a misreading of what nova_events is. There is no violation. |
| A `nova_gameplay_types` crate for shared markers | Owner chose layering; the dependency graph was verified acyclic, so the crate is unnecessary. YAGNI. |
| A single `NovaProbePlugin` spanning all three stages | Physically impossible across the process boundary. |
| `trait Capability` covering configuration | The three configs do not unify. Declare+arm only, if at all. |
| Test coverage as a success criterion | Owner's explicit instruction: separate task, do not create it. |
| Rewriting AGENTS.md at the end | It is the file every agent reads first; a stale map costs the whole epic. |
| A `clippy::pedantic` CI job | Measured 2026-08-07: 66% of its 3,998 warnings here are `needless_pass_by_value` (fires on **every** Bevy system parameter) and `redundant_pub_crate` (tells you to drop a `pub(crate)` this repo writes deliberately). Both are wrong for a Bevy codebase; the signal is unreadable without allowing them. |
| A `cast_*` truncation cleanup pass | 37 of clippy's 51 "real signal" hits. Sampled from two directions - clippy-side and grep-side - and every float-to-int cast read was clamped within 2 lines, several with a comment naming the reason. Measured clean. |
| Fixing the clippy style bucket (`map_unwrap_or` 190, `suboptimal_flops` 205, `wildcard_imports` 47, ...) in this epic | These are "is this our house style?" questions, which is what the CONVENTIONS workstream exists to answer. Route the list there; each candidate rule arrives with a free violation count from the 2026-08-07 run. |
| Deriving CONVENTIONS rules from what "sounds like good Rust" | Two independent sweeps measured the suspected patterns - `as` casts, float equality, division by zero, unwrap/panic as a class, unbounded indexing, duplicate system registrations, dead `pub` items - and found **0 genuinely bad** in nearly all of them. A rule adopted without a violation count will land on a clean codebase and produce only false positives. |

## L0 - what `--features debug` actually builds (F79, and half of F52)

Measured 2026-08-07 while gating the 11 dead default-feature example items.
F52 (L5) is the same question from the crate side; do not re-derive this half.

The feature is declared in **six** crates and fans out from `nova_core`:

| Crate | `debug =` |
| --- | --- |
| root | `nova_core/debug`, `bevy/track_location`; `dev` is an alias |
| `nova_core` | `nova_debug` (optional dep), `bevy/track_location`, `nova_editor/debug`, `nova_gameplay/debug`, `nova_info/debug`, `nova_menu/debug` |
| `nova_assets` / `nova_editor` / `nova_menu` | `bevy/track_location`, `nova_gameplay/debug` - pure forwarders, 0 `cfg` sites |
| `nova_gameplay` | `bevy/track_location` - the only crate with real `cfg` sites (13) |
| `nova_info` | `[]` - **a dead flag.** 0 sites, forwards nothing |

`nova_debug` declares no feature at all: it is "debug-gated" by being an
optional dependency of `nova_core`, while hard-forcing `nova_gameplay/debug` in
its own manifest.

**Where the sites actually are.** 402 of the 421 `cfg(feature = "debug")` sites
in the tree are in `examples/` - 24 of 26 example files. The library total is
19 (gameplay 13, core 4, probe 1, root `main.rs` 4 for `--norender`/
`--debugdump`). So `debug` is overwhelmingly an EXAMPLE-harness switch, not a
game switch, which is the shape F52 has to explain.

**Why the default-features CI job was needed at all.** Root lists `nova_debug`
as an unconditional dev-dependency, so every `cargo test` and every probe run
already compiles the debug graph. Nothing in the previous CI ever built the
examples with the feature OFF, and 11 items had rotted into that gap
undetected: 8 constants, 2 functions, 1 enum variant, plus 1 unused import.

**One item was not a clean gate.** `controller_section.rs`'s `Layout::B` is
constructed only under `debug` but pattern-matched in two ungated places, so it
needs the attribute on the variant, on its `match` arm, and on the `if
matches!` that pushes the off-axis hull. That last gate makes `ship_sections`
non-`mut` without the feature, hence the `cfg_attr(not(debug), expect(
unused_mut))` beside it. A cleaner fix is to stop threading `Layout` through
`attitude_rig` at all - out of scope here; note it for whoever touches the file.

## L0 - F80 measured, and the plan's count was wrong twice

The plan said 38 sites with 2 known-stale. Measured: **37** sites, and
converting all 37 to `#[expect(clippy::type_complexity, reason = "...")]` made
clippy report **12** unfulfilled expectations, not 2.

That is the whole argument for the rule, made on its first use: the two the
plan nominated by eye were `ammo_readout.rs:325` and `:510`, and only `:510`
was actually stale. `:325` is live. Ten more were dead and nobody had spotted
them. The 12 deleted:

`nova_ui/widget/panel.rs`, `nova_scenario/render_scale.rs`,
`nova_gameplay/audio/loops.rs`, `hud/ammo_readout.rs:510`,
`hud/component_lock.rs`, `hud/lock_crosshairs.rs`, `hud/nova_os/crt.rs` (x2),
`hud/nova_os_map/scene.rs` (x2), `hud/nova_os_ship/scene.rs`,
`hud/turret_lead.rs`.

25 conversions survive, plus the 4 pre-existing `#[expect]` sites = 29.

## L2 - the baseline ran, and three harness defects it found

Numbers, findings `B1`-`B6` and the harness recommendations `H1`-`H6` are in
`notes/18-benchmark-baseline.md`. Not repeated here. The headline for planning:
**tier 1 is at ceiling** (`blind` 0.97 at 40 tool calls for 30 questions), so it
is a regression guard and the after-run's headline has to be tier 2.

Three defects were found by running the lane's own review steps against real
artifacts rather than reading the code. All three would have corrupted the
number every structural lane is measured against, and none failed loudly:

1. **Tier 2 Completeness was scoring the sandbox.** The grader deducted from
   `blind` and `rustdoc` for not naming `CHANGELOG.md` and `web/src/wiki/**` -
   files deleted from those images. It was 100% confined to those two personas
   (9 and 7 entries; 0 for `tree`). Before the fix all four personas sat in
   0.763-0.787, indistinguishable; after it and the Cost of arrival fix below,
   `blind` 0.91 and `rustdoc` 0.88 lead the two derived channels (`tree` 0.74,
   `docs` 0.73). The bug was flattening the only ranking tier 2 produces.
   Fixed by `## Channel scope` in `keys/tier2.md`.

2. **A dropped question silently shrank the denominator.** `rustdoc` answered
   `t1-018`; the grader never returned it; `aggregate.py` computed `asked` from
   the grades, so 27 answers were averaged over 26. Now `asked` comes from the
   key and a gap prints UNGRADED QUESTIONS.

3. **`rustdoc`'s `[source]` hrefs outlived the pages they pointed at.**
   `../src/nova_mod_format/lib.rs.html#139` is a file:line answer at tier 1
   grain. The baseline never touched them, so it stands; `stage_rustdoc` now
   strips them so the after-run cannot.

The persona filter that decides all of this was implemented twice with nothing
catching drift. It is now `benchmark/persona_filter.py`, imported by
`make-papers.py` and shelled to by `grade.sh`.

4. **Cost of arrival was never computable.** It is a ratio against the owner's
   tool-call count; the owner works in an editor and has no transcript, so the
   denominator does not exist for any task. Eleven of twelve graders said so in
   their own citations and scored it anyway. A quarter of every tier 2 headline
   was unmeasured. Now null when unanchored; the headline is a 3-dimension mean.

5. **The grader was fed self-reported tool counts** for that same dimension.
   `blind/tier2a` self-reported 14 against 28 actual. `grade.sh` now counts via
   `aggregate.parse_transcript`.

### The grader noise floor bounds what the after-run can prove, and it is not uniform

`tree` and `docs` were graded three times on identical notes. Mean spread
**0.097** across 18 dimension-cells, but concentrated: Completeness 0.047 (max
0.08), Ownership 0.110 (max 0.27), No phantom structure 0.133 (max 0.25).

Completeness is stable because it counts against a Required list. The other two
are judgement calls against an anchor table and swing by up to 0.27 on identical
input. **Read the after-run delta from Completeness**; grade the other two k=3
and average (`H1`). Grading is a small container against the key with no source
tree, so three passes cost a fraction of one persona run.
