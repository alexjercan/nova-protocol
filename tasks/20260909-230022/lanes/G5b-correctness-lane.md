# G5b correctness lane

agent: agent-a9fa4529cdb067f7e
last_ts: 2026-09-09T20:50:11.250Z
stop_reason: end_turn
records: 251

## Dispatch prompt

```
You are the Correctness lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/correctness.md

Range under review: the rest of commit `7fdd25222` ("Draft the v0.13.0 news post with its media and widgets") outside the three biggest loop producers - the remaining examples, the capture scripts, and the workspace manifest.

Bundle, already built and PATH-SCOPED. Do NOT re-derive the range:
- /tmp/nightly-20260909/g5b/log.txt
- /tmp/nightly-20260909/g5b/stat.txt
- /tmp/nightly-20260909/g5b/diff.txt

Files: examples/screenshots/{loop_belt_compare,loop_death_compare,loop_sections_compare,screenshot_comms}.rs, examples/systems/{stress_hull_collapse,system_lock_line_of_sight}.rs, examples/playable/{first_shift_ships,railgun_wake_bench}.rs, scripts/capture-web-media.sh, scripts/gen-web-screenshots.py, Cargo.toml, crates/nova_debug/src/harness.rs, docs/development.md.

Focus on: the shell script (quoting, `set -euo pipefail`, failure that is swallowed, a loop that continues past a failed capture and reports success); the Python screenshot generator (path handling, silent skips); the 30 new lines in Cargo.toml (are all six new examples registered with the right `required-features`, and does every one of them build under DEFAULT features - a later commit `9e69ac196` had to add `#[cfg(feature = "debug")]` because `cargo check --all-targets` failed on six targets); and whether the changed `examples/systems/` ranges keep their `outcome:` contract with crates/nova_probe_cli/tests/catalog_drift.rs.

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do NOT run an example, a probe, or any benchmark.
- Never run the workspace test suite or Clippy. `nix develop --command cargo check --example <name>` and `cargo test -p nova_probe_cli --test catalog_drift` are allowed.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

## Findings

**MAJOR — scripts/gen-web-screenshots.py:242 (and :1060) — the four new `CUTS` sources are named by no producer, so the sweep never stages them and still exits 0.**

`producers()` walks `FIGURES` only. None of `asteroid-kinds-grid.png`, `planet-types-lineup.png`, `wfc-ships-row.png`, `first-shift-ships.png` has a `FIGURES` (or `THUMBNAILS`) row, so `asteroid_kinds`, `planet_types`, `wfc_ships` and `first_shift_ships` are never printed.

Failure scenario: run the documented re-cut path, `NOVA_UNFREEZE=news-0130 scripts/capture-web-shots.sh`. `capture-web-shots.sh:56` takes its example list from `--producers`, so those four benches never run, `target/shots/first-shift-ships.png` never exists, `build_cuts` (:826) prints `pending news-0130-block-fleet.png (source first-shift-ships.png not staged)` and appends to `pending`, which only feeds `pending_count`. `all_failed` stays empty and `main()` exits 0. All four `news-0130-*` cut figures silently keep whatever bytes are already in `web/src/assets`. `examples/playable/first_shift_ships.rs` was changed *in this commit* to shoot `first-shift-ships.png`, and nothing in the repo runs it with `NOVA_CAPTURE=1`.

This is the exact contract `producers()`'s own docstring states ("a figure whose producer was renamed then fails to be captured loudly, instead of quietly staying whatever was last copied") and that `docs/development.md:378` repeats.

Change: give `CUTS` a producer column (`(web_name, stage_source, example, window)`) and have `producers()` walk `CUTS` as well as `FIGURES`. Adding the bench frames to `FIGURES` is the wrong fix — they carry the seed readout and the site does not ship them.

Not higher: the four figures are shipped and currently correct, so nothing is broken on the live site today; the loss is the regeneration path.

---

**MAJOR — examples/systems/stress_hull_collapse.rs:217 — `NOVA_COLLAPSE_LOOP=0` turns the loop mode ON.**

```rust
fn loop_requested() -> bool {
    std::env::var_os(LOOP_ENV).is_some()
}
```

The two sibling switches added in the same commit parse the value and reject anything else — `railgun_wake_bench.rs:1030` (`loop_cut`) and `system_lock_line_of_sight.rs:123` (`sight_loop`) both `match raw.as_str()` on `"0"`/`"1"` and panic on `other`. This one is presence-only.

Failure scenario A: an operator who has `NOVA_COLLAPSE_LOOP` exported turns it off the way the sibling ranges document — `NOVA_COLLAPSE_LOOP=0` — and runs `cargo run --example stress_hull_collapse` under default features. `main()` takes the `loop_requested()` branch and hits the `#[cfg(not(feature = "debug"))] panic!("… needs --features debug")`. The range does not run at all.

Failure scenario B: same variable at `0` with `--features debug NOVA_CAPTURE=1`. `LoopCapturePlugin` is added and armed, which inserts `TimeUpdateStrategy::ManualDuration` (`crates/nova_autopilot/src/loops.rs:347`). Claims 4 and 5 (`the collapse frame cost is recorded`, `the debris the collapse threw is recorded`) then record the recorder's pinned frame step, not the collapse's — precisely the contamination the module doc at :50-57 warns about — while the operator believes the loop is off.

Change: match `"0"`/`"1"` and panic on anything else, as the two siblings do.

Not higher: it does not fail an unset-environment run, which is every CI and probe path.

---

**MINOR — scripts/capture-web-media.sh:309 — `package_import` skips the resolution gate `package_loop` enforces.**

`package_loop` (:261) rejects anything that is not `1280x720`. `package_import` checks only "file exists and non-empty" and the byte budget, then writes a `frozen` manifest row.

Failure scenario: a `-before-after` split composed by hand in content-machine at 1920x1080 (the resolution every producer shoots at, so the natural source size for a hand cut) is dropped into `web/src/assets/loops`. The sweep prints `>> news-0130-sections-before-after.webm: 8s, … (loop_sections_compare, imported)` and exits 0, and the site ships an off-spec loop. These are the only files in the flow that *cannot* be restaged, so the size gate matters more here than anywhere else, not less.

I confirmed all seven shipped imports are currently `1280x720`, so this is latent.

Change: run the same `ffprobe` width/height pair and the same `1280x720` assertion in `package_import`.

---

**MINOR — examples/systems/stress_hull_collapse.rs:1080 — the new recorded drift/spin observation carries no `probe_marker` and no roster slug.**

Every other reading in `verify()` is a `nova_probe::probe_marker` with an `outcome:` slug on the `crates/nova_probe_cli/tests/catalog_drift.rs` roster, including the two that only RECORD (`the collapse frame cost is recorded`, `the debris the collapse threw is recorded`). The new block emits a bare `info!`.

Failure scenario: the block is deleted, or `world.get::<LinearVelocity>(block)` returns `None` so the `if let (Some, Some)` silently records nothing. `systems_ranges_assert_their_invariant_roster` stays green and `probe run stress_hull_collapse` reports the same five outcomes — which is the exact hole `examples/systems/README.md` ("Every claim is named": "a slug can name a recorded observation rather than a claim") puts the roster there to close.

Change: emit it as `probe_marker(world, "outcome: the shell's drift after the collapse is recorded", …)` and add the slug to the `stress_hull_collapse` roster at catalog_drift.rs:513.

Not higher: nothing reads the number today.

---

**MINOR — scripts/capture-web-media.sh:77 — three of the six new producers are on no roster in the capture flow.**

`loop_sections_compare`, `loop_belt_compare` and `loop_death_compare` are registered in `Cargo.toml` and each writes `news-0130-{sections,belt,death}-after.webm`, but none appears in `LOOPS`, `ALIASES` or `PENDING`, and no `*-after.webm` name appears anywhere in the script. Only the hand-composed `*-before-after` splits are listed, under `IMPORTED`, whose failure message is `it is cut by hand, so nothing can restage it`.

Failure scenario: the v0.13.0 "after" halves need re-cutting. `scripts/capture-web-media.sh` runs the whole set, reports `N loop(s) in …` with a success exit, and produces none of the three. The only record of how to produce them is each example's own module doc. `docs/development.md:369-371` states that `capture-web-media.sh` and `gen-web-screenshots.py` "name every producer and every file it writes"; after this commit they do not.

Change: list the three under `PENDING` (with the "composed by hand from a content-machine capsule" reason) or add them to `LOOPS` staging `news-0130-*-after` so the halves are at least reproducible.

Not higher: `probe run --all` still spawns all three, so they cannot rot silently as *code*; the loss is only the media path.

---

**MINOR — examples/playable/first_shift_ships.rs:54 — the new capture wiring is not gated, so the documented hand-run loses its dev overlays and a resizable window.**

```rust
app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
```

Neither helper is env-gated (`crates/nova_debug/src/harness.rs:651`, `:693`).

Failure scenario: `cargo run --example first_shift_ships --features debug`, the run the module doc advertises at :16-19 ("Hand-run with the free WASD camera") for "a free-fly visual review" of silhouettes. The window is forced to 1920x1080 and `resizable = false`, and `DebugEnabled`, `inspector::DebugEnabled` and `wireframe::DebugEnabled` are all cleared — the section overlay and wireframe pass a silhouette review would use. `Cargo.toml:37-41` puts `playable/` examples' affordance outside the harness gate.

Change: `if capturing() { app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays)); }`, the way `railgun_wake_bench.rs` already gates `hide_hud`.

Not higher: `railgun_wake_bench` and the ungated `freeze_bodies` already in this same file do the same thing, so this is an existing house habit rather than a new one — but this commit is what applied it to a hand-review bench.

---

**MINOR — docs/development.md:368 — the list of examples that "ship a loop beside their stills" is stale as of this commit.**

It still reads `screenshot_railgun`, `screenshot_editor`, `wfc_arena` and `system_torpedo_launch`. This commit added `loop_start`/`loop_end` pairs to `railgun_wake_bench`, `stress_hull_collapse` and `system_lock_line_of_sight` — the last two being `systems/` correctness ranges, which is the interesting half of the claim and is now unstated. The commit edited the adjacent stills list two lines up (to add `screenshot_comms`) and not this one.

---

**MINOR — scripts/gen-web-screenshots.py:236-241 — the `CUTS` comment contradicts the row directly under it.**

The comment says "A post figure has no use for the readout, so its window starts under it", then :243 gives `news-0130-asteroid-kinds.png` the window `(0, 0, 1920, 1080)` — the whole frame, readout included. A full-frame "cut" is also just an alias with a decode/re-encode round trip; the other three windows do start under the readout. Either move that row to `ALIASES` or fix the comment to say the asteroid grid keeps its readout and why.

---

`Checked:`
- Read `AGENTS.md`, `.agents/skills/nova-review/lanes/reviewer.md`, `lanes/correctness.md`, `examples/systems/README.md`, and the bundle at `/tmp/nightly-20260909/g5b/{log,stat,diff}.txt` in full.
- **Cargo.toml, all six new `[[example]]` blocks**: `loop_hull_generate`, `screenshot_comms`, `loop_goto_standoff`, `loop_helm_orders`, `loop_sections_compare`, `loop_belt_compare`, `loop_death_compare` are all present with correct `path`s. No example in the file declares `required-features` — the house pattern is default-features build plus internal `#[cfg(feature = "debug")]`, so "none" is the right answer here and these match. `cargo test -p nova_probe_cli --test catalog_drift` passes (`catalog_matches_disk`, `systems_ranges_assert_their_invariant_roster`), which pins catalog == disk.
- **Default-features build**: `nix develop --command cargo check --keep-going` over all eleven touched/added examples — clean, exit 0. The `cargo check --all-targets` break the brief names was real at 7fdd25222 and is fully repaired by 9e69ac196; nothing in this range still fails without `debug`. Verified 9e69ac196's gating follows the consumer (`LOOP_AFTERMATH_SECS` correctly stays ungated because `close_the_loop` is ungated; `loop_sections_compare`'s `LOOP_SECS`/`START_YAW`/`TurnClock` correctly stay ungated for `turn_bench`).
- **`outcome:` contract**: rosters for `stress_hull_collapse` (catalog_drift.rs:513, five slugs) and `system_lock_line_of_sight` (:427, four slugs) are unchanged and still match their markers both ways. `sight_loop_script` adds no markers and deletes none; the default assertion path is untouched. Confirmed `system_lock_line_of_sight` under `NOVA_SIGHT_LOOP=1` runs zero assertions by design and that `capture-web-media.sh` scopes that variable per-producer with `env`, so no probe or CI path inherits it.
- **`stress_hull_collapse` loop-mode exit path, traced end to end**: `SETTLE_SECS = 8.0 > LOOP_AFTERMATH_SECS = 6.0`, both measured from `window_opened`, so `close_the_loop` always fires before `verify()` sets `verified` and moves the walk into the branch that never calls it again. No hang. `the_loop_is_written` returns `true` unarmed (`predicate.rs:215`), and `completion::register` installs the watcher, so the unarmed loop mode still exits. The 120 s completion backstop counts `Time<Real>`, which `ManualDuration` itself pins during a capture, so it is 3600 rendered frames and not 120 wall seconds — no deadline risk.
- **Shell script**: `set -euo pipefail` present; the LOOPS run loop cannot continue past a failed capture (`[[ -s "$file" ]] || exit 1` at :208); the alias-source `news-*` guard at :149-156 still holds for all ten new aliases; the new `IMPORTED`/`PENDING` records contain no `|` so `IFS='|' read` parses them; new `LOOPS` rows with empty args take the correct `else` branch; `duration=$(...)` assignments abort under `set -e`. `ffprobe`-verified all seven `IMPORTED` webms are 1280x720 and under the 3 MB budget.
- **Python**: `crop_window` and `opaque_rgba` are correct for 3- and 4-channel buffers (`decode_png` rejects everything else); all four `CUTS` windows are in-bounds against 1920x1080 and within `ASPECT_TOLERANCE` of 16:9; `manifest_owners` declares `CUTS`; `build_cuts` pending tuples match the `build_composites` shape; `frozen()` correctly holds all four now that they ship.
- **`screenshot_comms` content ids**: `CommsPanelHUD` matches `comms_panel.rs:210`; the panel spawns no permanent children so `children.len() >= 3` counts cards only, and `COMMS_VISIBLE_CAP` is 3; `comms`/`crew`/`guard` all exist in `assets/base/channels/base.content.ron` with the tones the module doc claims; `COMMS_DWELL_MAX_SECS` is 30.0; `base/portraits/range-control.png` exists and is the correct merged rewrite of `self://portraits/range-control.png` (base's `resource_base` is `"base"`).
- **`loop_belt_compare` scatter density**: simulated the rejection sampler (400 seeds, 24 rocks, 84 m separation, 64 attempts, the authored 140x180x720 m box) — mean 23.86 placed, 0.14 dropped. `ROCKS_PER_FIELD = 24` is honoured; the two boxes are 120 m apart in x so the shared `scatter_placements` list causes no cross-field rejection. Not a finding.
- Confirmed the unconditional `LoopCapturePlugin::default()` in `railgun_wake_bench` is the fleet-wide idiom (23 other examples do the same, including `screenshot_railgun` and `screenshot_editor`, which also shoot stills). Not a finding.

`Not checked:`
- **I ran no example, probe or benchmark** — I do not hold the measurement slot. Every claim about what a producer actually records, how a frame looks, whether a loop closes on itself, or whether the encoder stays under budget on a real capture is unverified. Specifically unverified: the `LOOP_SLOW_AT = 60.0` / `LOOP_PASSED_AT = 156.0` two-frame margin in `railgun_wake_bench` (the arithmetic in its doc comment is self-consistent, but the "25 units per real frame" premise is a measurement); `LOOP_CRF = 40` keeping `news-0130-hull-collapse` under 3 MB on a re-cut (the shipped file is 2.55 MB, 85% of budget); `COVER_DRIFT_SECS`'s claim that the whole crossing fits one loop.
- Did not run Clippy or the workspace suite, per the contract.
- The three biggest loop producers (`loop_hull_generate`, `loop_goto_standoff`, `loop_helm_orders`) are outside this range — I read only the `Cargo.toml` rows and `capture-web-media.sh` entries that name them, and 9e69ac196's gating of them. Their bodies were not reviewed.
- `web/src/**` (the news post itself, `web/tests/assets.test.js`, the widgets) is another lane's; I did not check whether every `news-0130-*` name the post references is shipped, nor whether the post reads a `news-` loop from outside its own namespace.
- The `PENDING` row for `news-0130-goto-standoff` that this commit added was resolved downstream by `a5f7df1d6`, which put `loop_goto_standoff` into `LOOPS` and emptied `PENDING`; I reviewed the reviewed-commit state but did not audit `a5f7df1d6` itself.
- `web/src/assets/loops/manifest.txt` is generated output and I did not treat its drift as a finding, but note for the record that rows 52-54 (`news-0130-*-before-after`, listed as `fresh` with an example in the source column) are not in the format `package_import` writes, so the shipped manifest was not produced by an end-to-end run of the current script.
- Did not verify `hollow::ambush_hollow`'s engage-grace duration against `screenshot_comms`'s ~1.5 s script, so its "the frame behind the cards is a quiet range" claim is unconfirmed.
- Did not review `crates/nova_debug/src/harness.rs`'s one-line change beyond confirming `completion` is re-exported for `stress_hull_collapse`'s use.
