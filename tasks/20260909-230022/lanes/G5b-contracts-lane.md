# G5b contracts lane

agent: agent-a1a5f1ee5786f579e
last_ts: 2026-09-09T20:48:55.572Z
stop_reason: end_turn
records: 224

## Dispatch prompt

```
You are the Contracts lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/contracts.md

Range under review: the rest of commit `7fdd25222` ("Draft the v0.13.0 news post with its media and widgets") outside the three biggest loop producers.

Bundle, already built and PATH-SCOPED. Do NOT re-derive the range:
- /tmp/nightly-20260909/g5b/log.txt
- /tmp/nightly-20260909/g5b/stat.txt
- /tmp/nightly-20260909/g5b/diff.txt

Files: examples/screenshots/{loop_belt_compare,loop_death_compare,loop_sections_compare,screenshot_comms}.rs, examples/systems/{stress_hull_collapse,system_lock_line_of_sight}.rs, examples/playable/{first_shift_ships,railgun_wake_bench}.rs, scripts/capture-web-media.sh, scripts/gen-web-screenshots.py, Cargo.toml, crates/nova_debug/src/harness.rs, docs/development.md.

Pay particular attention to: whether every new `NOVA_*` environment variable is in docs/environment-variables.md and passes tests/env_contract.rs; whether each new loop is registered in web/src/assets/loops/manifest.txt with a row that matches what the capture script produces, and whether any manifest row is left PENDING with no artefact; whether examples/systems/README.md's `system_`/`stress_`/`bug_` naming and `outcome:` marker contract is honoured by the changed ranges and enforced by crates/nova_probe_cli/tests/catalog_drift.rs; whether docs/development.md's instructions still match the scripts after the change; and whether any figure printed for a reader is in meters per AGENTS.md.

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do NOT run an example, a probe, or any benchmark.
- Never run the workspace test suite or Clippy.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

## Findings

**MAJOR — docs/environment-variables.md:176 — three new example-local `NOVA_*` knobs are missing from the index that enumerates every one**

The "Example-local knobs" bullet names each such variable (`NOVA_STRESS_PD_*`, `NOVA_EDITOR_FRAMELOG`, `NOVA_SOAK_SCENARIO`, `NOVA_SOAK_SECS`, `NOVA_VFX_RANGE_BARE_SLUG`, `NOVA_RAILGUN_AFTERMATH`, `NOVA_RAILGUN_LIVE`) and, before this commit, that list was complete against `examples/`. The commit adds `NOVA_WAKE_LOOP` (examples/playable/railgun_wake_bench.rs:986), `NOVA_COLLAPSE_LOOP` (examples/systems/stress_hull_collapse.rs:191) and `NOVA_SIGHT_LOOP` (examples/systems/system_lock_line_of_sight.rs:94), and documents none of them. `tests/env_contract.rs` cannot catch this: its scan walks only `crates`, `src`, `tests` (line 179) and its own comment says `examples/` is deliberately absent (lines 116-117).

Failure scenario: a developer reads `scripts/capture-web-media.sh:105` setting `NOVA_COLLAPSE_LOOP=1`, looks it up in the index that claims to be "the index of the whole set", and finds nothing — while the same bullet spends five lines analysing the railgun pair's failure mode ("Neither goes wrong quietly"). The new trio has one that *does* go wrong quietly (next finding), and the analysis that would have caught it is exactly what the missing rows would have forced.

Change: add the three to the bullet at docs/environment-variables.md:176, saying which loop each records. `NOVA_STANDOFF_TRACE` (examples/screenshots/loop_goto_standoff.rs, another lane's file in the same commit) is missing for the same reason.

Not a BLOCKER: `env_contract.rs` is green by design and no run breaks; the cost is discoverability.

---

**MAJOR — examples/systems/stress_hull_collapse.rs:217 — `NOVA_COLLAPSE_LOOP=0` turns the loop mode ON**

```rust
fn loop_requested() -> bool {
    std::env::var_os(LOOP_ENV).is_some()
}
```

Both siblings added in this same commit parse the value and refuse anything else — `loop_cut()` at examples/playable/railgun_wake_bench.rs:998 and `sight_loop()` at examples/systems/system_lock_line_of_sight.rs:120 both `match raw { "0" => false, "1" => true, other => panic!("{LOOP_ENV}={other:?} must be 0 or 1") }`, copying the established `live_cut()` at examples/screenshots/screenshot_railgun.rs:217. Only the collapse range uses presence.

Failure scenario: `NOVA_COLLAPSE_LOOP=0 cargo run --features debug --example stress_hull_collapse` — the spelling anyone would reach for to turn the mode *off* — adds `LoopCapturePlugin`, pins the clock and records a 7 s webm, and the milliseconds claims 4 and 5 report become the recorder's rather than the collapse's. That is precisely the contamination the new module doc at line 52-61 warns about. With `NOVA_COLLAPSE_LOOP=0` and no `--features debug`, the run panics at line 307 instead of starting.

Change: parse the value the way `loop_cut`/`sight_loop` do. AGENTS.md's explicit-authoring rule ("an unrecognized id is an error") points the same way.

Not higher: nothing in `scripts/` or CI sets it to `0`, so no shipped run takes the branch today.

---

**MAJOR — web/src/assets/loops/manifest.txt:1,52-54 — the manifest was hand-appended and does not match what the capture script writes**

Three concrete disagreements with `scripts/capture-web-media.sh`:

1. Line 1 still reads `# captured at commit a5da7efdb on master`. `a5da7efdb` is an ancestor of this commit, and 22 rows for footage produced *at* this commit sit under that header. The header is written by the script (line 242) on every sweep, so it can only be right if the sweep ran.
2. Lines 52-54 (`news-0130-sections-before-after`, `-belt-`, `-death-`) carry state `fresh`. `package_import` writes the literal `frozen` for exactly those three names (scripts/capture-web-media.sh:322-323), and `frozen` is the truthful value: `fresh` means "re-copied from the stage this run", and nothing stages a hand-composed split — `package_import` exits 1 if the file is not already in `OUT` (lines 312-315).
3. Row order does not match the generator's write order (LOOPS, then ALIASES, then PENDING, then IMPORTED). `news-0130-hull-collapse` and `news-0130-helm-orders` are LOOPS rows but sit at lines 61-62, after the bench-replay imports.

Failure scenario: the next `nix develop -c scripts/capture-web-media.sh` rewrites `manifest.txt` wholesale — the three import rows flip `fresh` → `frozen`, the header sha moves and ~20 rows re-order — producing a diff indistinguishable from a real re-cut. Until then, the file whose only job is provenance tells a reader that three hand-composed splits were captured at `a5da7efdb`.

Change: regenerate from a sweep, or at minimum set those three rows to `frozen` and refresh the header sha.

Not a BLOCKER: nothing builds or renders off the state column; it is shipped metadata.

---

**MAJOR — scripts/gen-web-screenshots.py:236-246 — the new `CUTS` table reintroduces the hand-kept producer array `--producers` exists to prevent**

`producers()` (lines 1060-1071) walks `FIGURES` only. None of the four `CUTS` sources' producers — `asteroid_kinds`, `planet_types`, `wfc_ships`, `first_shift_ships` — appears in `FIGURES`, so `gen-web-screenshots.py --producers` never prints them and `scripts/capture-web-shots.sh:56` with no arguments never runs them. Meanwhile `manifest_owners()` (lines 965-966) declares each cut `capturable`, so the coverage report asserts a producer exists.

Failure scenario: `news-0130-block-fleet.png` is cut from `first-shift-ships.png`, a still this commit taught `first_shift_ships` to write. Delete the shipped PNG (or add a fifth cut before its figure ships) and run `scripts/capture-web-shots.sh`; `frozen()` (line 92-98) no longer short-circuits, `build_cuts` prints `pending news-0130-block-fleet.png (source first-shift-ships.png not staged)`, and the figure cannot be produced by the flow — the only place naming `first_shift_ships` as the thing to run is the comment at line 237. docs/development.md:378-379 still claims "`--producers` prints the list the site actually consumes, so a capture flow never runs off a hand-kept array."

Change: map each `CUTS` source file to its example and have `producers()` emit those too, so the four benches are reachable from the flow rather than from a comment.

Not higher: the four figures are shipped and frozen, so today's site is correct.

---

**MINOR — docs/development.md:365-371 — the media-flow claims went stale with this commit**

Two sentences in the `screenshots/` paragraph:

- "so `screenshot_railgun`, `screenshot_editor`, `wfc_arena` and `system_torpedo_launch` each ship a loop beside their stills" — the commit adds three more: `stress_hull_collapse`, `system_lock_line_of_sight` and `railgun_wake_bench`.
- "What a run makes is decided by `scripts/capture-web-media.sh` (loops) and `scripts/gen-web-screenshots.py` (stills), which name every producer and every file it writes." — `loop_sections_compare`, `loop_belt_compare` and `loop_death_compare` are registered in Cargo.toml and each writes a loop (`news-0130-sections-after`, `news-0130-belt-after`, `news-0130-death-after`); none of those six names appears in either script, in any `LOOPS` or `PENDING` row, or anywhere under `web/`. The only pointer is the free-text `source` column of the `IMPORTED` table (scripts/capture-web-media.sh:304-306).

Failure scenario: a `-before-after` split is lost or re-cut. `package_import` exits 1 with "it is cut by hand, so nothing can restage it"; a reader following development.md looks for the producer in the two scripts that "name every producer", where it is not.

Change: name the three compare producers in `capture-web-media.sh` (a comment row beside `IMPORTED` suffices) and correct the two sentences.

---

**MINOR — examples/systems/stress_hull_collapse.rs:1080-1094 — the new recorded observation carries no `outcome:` slug**

`examples/systems/README.md:47-58` makes every claim named by a `nova_probe::probe_marker` reading `outcome: <slug>` on the roster in `crates/nova_probe_cli/tests/catalog_drift.rs`, and says explicitly that a slug may name a recorded observation rather than an assert. This range already does that twice — "the collapse frame cost is recorded" and "the debris the collapse threw is recorded" (catalog_drift.rs:518-519), both emitted through `probe_marker`. The new drift/spin reading, labelled "RECORDED, never asserted" in its own comment, is an `info!` only.

Failure scenario: delete the block and every test stays green — `catalog_drift` matches roster slugs against markers, so an unmarked reading is invisible to it, and `probe run stress_hull_collapse` never carries the number into its report either. The reading the comment argues is worth keeping is the one nothing holds down.

Change: emit it via `probe_marker` and add its slug to the roster (`SYSTEMS_INVARIANTS` 244 → 245), or state in the module doc why it is a log line and not a recorded claim.

The units in that line are correct: `MetersPerSecond::from_engine(drift).0` at the avian boundary, with the boundary named in the comment beside it.

---

## Checked

- **Every new `NOVA_*` against `tests/env_contract.rs`**: green, and correctly so — the roster scan covers `crates`, `src`, `tests` only (line 179), and the file's own comment (lines 116-117) says `examples/` is excluded on purpose. Reported as a doc gap, not a test gap.
- **The pending-loop contract, end to end**: `news-0130-goto-standoff` is consistent at this commit — `PENDING` row in the script, `news-0130-goto-standoff.webm  -  0.0  0  pending` in the manifest, no artefact in `web/src/assets/loops/`, and the post renders a `figure__placeholder` (web/src/news/0.13.0.md:631-639) rather than a broken `<video>`. No manifest row is pending with a shipped artefact, and no shipped artefact lacks a row. A later commit (`a5f7df1d6`) promoted it to a `LOOPS` row.
- **Every other new loop's manifest row against the script**: `news-0130-railgun-wake`, `-hull-collapse`, `-lock-occlusion` each have a `LOOPS` row whose example column matches the manifest; all nine new alias rows source a living loop, so the "a news loop is never an alias source" guard (lines 149-156) passes. All rows are within `MAX_BYTES` (worst: `news-0130-hull-collapse` at 2 550 360 of 3 145 728, which is what `LOOP_CRF = 40` at stress_hull_collapse.rs:211 buys).
- **Resolution of the imported loops**: `package_import` skips the `1280x720` check that `package_loop` enforces; I ffprobed all seven imports and every one is 1280x720, so the gap is latent, not live. Not reported.
- **`examples/systems/README.md` naming and `outcome:` contract**: `stress_` and `system_` prefixes still correct for both changed ranges; no slug added, removed or renamed, so `catalog_drift`'s `SYSTEMS_INVARIANTS = 244` and both rosters stay accurate. Both clap `about` strings still say "Autopilot-only correctness range". `system_lock_line_of_sight`'s loop path runs zero assertions, which the module doc states outright (lines 27-33), and CI's `probe run systems` never sets `NOVA_SIGHT_LOOP`.
- **`LOOP_AFTERMATH_SECS` (6.0) against `SETTLE_SECS` (8.0)**: `close_the_loop` is unreachable once `verified` (drive_range returns early), so the loop must close before `verify` or the run hangs on `the_loop_is_written`. The 2 s margin holds and the constant's docstring names the dependency ("short of the settle the reading waits for"). No finding.
- **Smoke paths of all four new producers**: `loop_written` (predicate.rs:214-217) and `shot_written` (196-201) return a constant-true predicate when `NOVA_CAPTURE` is unset, and `loop_start`/`loop_end` are no-ops, so `probe run screenshots` will not hang on the new close steps. `completion::register` adds its own watcher on first registration (completion.rs:149-167), so `stress_hull_collapse` — which has no `AutopilotPlugin` — does get an exit decider under the loop mode.
- **Meters, every reader-facing figure**: all camera poses go through `pose_camera(world, Meters3, Meters3)` (harness.rs:748); `COVER_DRIFT_FROM`/`CoverDrift` convert at the transform with `to_engine()`; the only printed number is the drift line, converted with `from_engine` and commented as an engine boundary. `ROCK_SEPARATION = Meters(84.0)` checks out against `ASTEROID_GEOMETRIC_FACTOR_MAX = 6.0` (2 × 7 m × 6). `railgun_wake_bench`'s `LOOP_SLOW_AT`/`LOOP_PASSED_AT` are bare `f32` world units, but each docstring says "Engine world units" and they feed `slug_flew`, an engine-side predicate — within the AGENTS.md carve-out.
- **`force_capture_resolution` on `first_shift_ships`**: added ungated, which changes a hand-run's window, but that is the fleet idiom — `asteroid_kinds`, `planet_types`, `compare_planets`, `compare_asteroids`, `greeble_catalog`, `shape_bench` and `railgun_wake_bench` all do the same at Startup. Not a finding.
- **Default-features `-D warnings` (`cargo check --workspace --all-targets`, ci.yaml:308-313)**: at this commit `loop_belt_compare`'s `DRIFT_FROM`/`DRIFT_TO`/`LOOK_AHEAD`/`LOOP_SECS`/`DriftClock`, `loop_death_compare`'s `KILLED_CELL`/`EYE`/`LOOK`/`INTACT_SECS`/`AFTERMATH_SECS`, and `loop_sections_compare`'s `EYE`/`LOOK` were reachable only from `#[cfg(feature = "debug")]` code but not themselves gated — dead-code warnings that would have failed that job. **Already fixed** by `a5f7df1d6` in the tree; not raised as a live finding.
- **`crates/nova_debug/src/harness.rs:113`**: the `completion::{self, …}` re-export is additive, reaches the same protocol instance the drivers register with, and `stress_hull_collapse` consumes it through the sanctioned `nova_debug::harness` path.
- **Cargo.toml example blocks**: all seven new producers have one, so `catalog_matches_disk` stays green.
- **`CUTS` geometry**: all four windows are within `ASPECT_TOLERANCE = 0.02` of 16:9 and inside a 1920x1080 frame; `crop_window`/`opaque_rgba` index correctly for 3- and 4-channel buffers.

## Not checked

- **Anything requiring a run.** No example, probe or benchmark was executed — I do not hold the measurement slot. So: whether the three compare producers actually record a loop that closes on itself, whether `NOVA_COLLAPSE_LOOP=1` finishes inside the 120 s completion deadline on lavapipe (the recorder pins `Time<Real>` to the frame step, so the backstop counts rendered frames — this is the one I would most want measured), and whether `system_lock_line_of_sight`'s drift crossing actually breaks the lock on the recorded path.
- **`cargo check` of any kind.** No compile was run; the default-features conclusion above is from reading the cfg attributes against ci.yaml, and is stated as already-fixed rather than as a live claim.
- **No workspace test suite and no Clippy**, per the reviewer contract.
- **The three big loop producers** (`loop_hull_generate`, `loop_goto_standoff`, `loop_helm_orders`) and `web/src/news/0.13.0.md` itself — outside my path scope. I read the post only to resolve the `news-0130-goto-standoff` placeholder question and to enumerate its figure references.
- **Cross-release determinism of the belt.** `loop_belt_compare`'s module doc claims the field is "the same belt rock for rock" as the v0.12.0 content-machine capsule `belt-compare`. That depends on the capsule using the same `count`, `min_separation` and region, and on v0.12.0's position stream being untouched by the salted kind stream. The capsule lives outside this repo; I verified only that `ScatterObjectsConfig` draws positions, silhouettes and kinds from three separately salted streams (spawn.rs:376-390) and that a rejected sample still advances the position RNG.
- **Rock yield of the belt scatter.** 24 rocks at 84 m centre-to-centre in a 140 × 180 × 720 m box is near the random-sequential-adsorption limit, and `sample_clear_of` DROPS a copy that cannot clear in 64 attempts, logging at `trace!` (spawn.rs:455-465). Whether both halves land all 24 — and land the same count as each other — needs a run to settle.
- **`CHANGELOG.md`.** Not in the bundle's path scope and not touched by this commit; I did not judge whether the v0.13.0 post's subject matter needs an entry.
