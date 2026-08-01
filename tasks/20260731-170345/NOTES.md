# NOTES - 20260731-170345

KISS pass on the rest of `crates/nova_gameplay/src/` - flight, camera,
gravity, juice, audio, damage, settings, plugin, lib and the small siblings.
Both axes: the three oversized files split into folder modules, comment
rubric applied across every file in scope.

## Structure axis

Before: 3 files held 9828 of the area's ~13.8k lines.

| Before | Lines |
|-|-|
| flight.rs | 5812 |
| audio.rs | 2264 |
| camera_controller.rs | 1752 |
| gravity.rs | 1060 |
| juice.rs | 1051 |

Each split is `mod`-level only: a folder module whose `mod.rs` keeps the
module doc, the plugin, the system set and the prelude, plus one sibling file
per concern. Public paths are unchanged - each `mod.rs` re-exports every name
the pre-split file made visible at the parent path, so `flight::Autopilot`,
`flight::RcsReference`, `camera_controller::SpaceshipCameraControlMode` and
`audio::SfxListenerMarker` still resolve exactly as before, and both
`flight::prelude` and `camera_controller::prelude` are byte-identical to
master. No call site outside the three modules needed repointing.

Re-exports are explicit name lists, not globs: a `pub(crate) use self::x::*;`
glob is what produces the `ambiguous import visibility` warnings the
`nova_os_*` modules already carry, and this pass should not add more.

### After

Prod/tests measured at each file's first `#[cfg(test)]` boundary.

| File | Lines | Prod | Tests | Concern |
|-|-|-|-|-|
| flight/autopilot.rs | 939 | 939 | 0 | the one autopilot system: STOP/GOTO/GotoPos/ORBIT, the RCS handoff, engine cool-down |
| flight/guidance.rs | 636 | 340 | 296 | pure arrival/orbit math and the slew helpers |
| flight/thrusters.rs | 532 | 288 | 244 | thruster clustering, group choice, the balance QP, spool |
| flight/state.rs | 390 | 390 | 0 | components, telemetry, the reflected `FlightSettings` |
| flight/manual.rs | 299 | 299 | 0 | manual burn, the RCS primitive, player intent decay |
| flight/mod.rs | 143 | 44 | 99 | module doc, plugin, system set, prelude, re-exports |
| flight/tests/manual.rs | 527 | - | 527 | manual burn + balancer physics |
| flight/tests/rcs.rs | 434 | - | 434 | the RCS verb end to end |
| flight/tests/goto.rs | 471 | - | 471 | GOTO / GotoPos arrival |
| flight/tests/support.rs | 424 | - | 424 | shared app rigs and spawn helpers |
| flight/tests/stop.rs | 367 | - | 367 | the STOP verb |
| flight/tests/orbit.rs | 288 | - | 288 | the ORBIT verb |
| flight/tests/control.rs | 223 | - | 223 | controller authority and the command clock |
| flight/tests/telemetry.rs | 216 | - | 216 | published `ManeuverTelemetry` |
| flight/tests/mod.rs | 38 | - | 38 | test-module doc + the intent-insert test |
| audio/loops.rs | 729 | 355 | 374 | the continuous thruster hum and RCS loops |
| audio/combat.rs | 543 | 227 | 316 | explosion, impact, turret fire, torpedo launch one-shots |
| audio/cues.rs | 432 | 168 | 264 | cockpit cues: lock, safety, dry fire |
| audio/mixing.rs | 343 | 198 | 145 | listener, falloff, volumes, the per-cell throttle |
| audio/mod.rs | 313 | 49 | 264 | module doc, `UiSfx` keys + volumes, plugin, prelude |
| audio/test_support.rs | 14 | 14 | 0 | the two observer resources every cue test asserts through |
| camera_controller/mode.rs | 660 | 230 | 430 | mode/stance derivation and look-input routing |
| camera_controller/framing.rs | 533 | 235 | 298 | the chase anchor, per-mode rig, burn push, survey dolly, velocity lead |
| camera_controller/handback.rs | 290 | 100 | 190 | the autopilot-to-manual blend |
| camera_controller/rig.rs | 248 | 248 | 0 | the controller entity, its rigs, markers and input actions |
| camera_controller/mod.rs | 116 | 116 | 0 | module doc, plugin, system set, prelude, re-exports |
| gravity.rs | 1048 | 427 | 621 | unchanged shape - one concern |
| juice.rs | 1050 | 629 | 421 | unchanged shape - one concern |
| damage.rs | 559 | 326 | 233 | unchanged shape |
| settings.rs | 548 | 322 | 226 | unchanged shape |
| plugin.rs | 223 | 175 | 48 | unchanged shape |
| lib.rs | 166 | 39 | 127 | unchanged shape |
| asset_ref.rs | 162 | 132 | 30 | unchanged shape |
| relations.rs | 100 | 75 | 25 | unchanged shape |
| objective_marker.rs | 58 | 58 | 0 | unchanged shape |
| beacon.rs | 31 | 31 | 0 | unchanged shape |

No file exceeds 1500 lines, so DoD 4 needs no exception.

### Three judgement calls

- **`flight/tests/` is a folder, not per-file test modules.** ~2800 of
  flight.rs's lines were physics-level integration tests that build the whole
  plugin and assert on hull motion - they do not belong to any one production
  file. They moved as a folder module split by verb, with the 18 shared rig
  helpers in `tests/support.rs`. Only the pure-math tests stayed co-located,
  in `guidance.rs` and `thrusters.rs` where the functions they cover live.
- **`audio/test_support.rs` exists** because `PlayedSfx` and `LastPlayed` are
  asserted through by both `combat.rs` and `cues.rs`. It is `#[cfg(test)]`-only
  and mirrors the crate's existing `integrity::test_support`, so it is not a
  new abstraction.
- **`gravity.rs` and `juice.rs` were left whole.** Both are ~1050 lines, both
  under the limit, and each is one concern (one plugin, one system, its
  components and its tests). Splitting them would be motion, not
  simplification. Only the comment axis touched them.

### Visibility

Items that stayed private before now cross a module boundary. Each was
widened to the narrowest visibility that works: `pub(super)` for
module-internal systems, constants and the split-out helpers; `pub(crate)`
kept only where an out-of-module caller already existed
(`flight::is_forward_aligned`, `ship_turn_rate`, `slew_rotation`,
`accumulate_rcs_axis`). No item became `pub`; the crate's external API is
byte-identical.

`flight::hull_turn_rate` is re-exported under `#[cfg(test)]` only - its two
callers (`input/ai/maneuver.rs`, `input/player/intent.rs`) are both test call
sites, and an unconditional re-export would warn in the lib target.

## Comment axis

Provenance stripped everywhere: tatr IDs, bare dates (`2026-07-10 playtest`),
spike/decision/review labels (`spike decision 3`, `Q4`, `R1.1`, `round-3 M2`,
`PR #54`) and every `docs/spikes/*.md` path (`docs/` is ephemeral scratch per
AGENTS.md, so those pointers rot). The prose that carried them stays - none of
the surviving explanation depended on the citation. 201 comment lines carried
a provenance clause before this pass.

`grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the scope returns **zero** hits, so
DoD 3's list is empty: no deferred work in this area has a live backlog task
worth citing.

Six comments were promoted to `NOTE:` - each guards a tuned value that a
future edit would otherwise silently break:

| Site | Guards |
|-|-|
| settings.rs:198 | render_scale 0.7 - measured ~neutral on the dev GPU, kept for weaker fill-bound web hardware |
| gravity.rs:114 | `default_surface_gravity` 6.0 - doubled after playtest, the arrival solver budgets the pull |
| gravity.rs:126 | `soi_factor` 8.0 - retuned from 4.0 so the well announces itself |
| damage.rs:99 | the resistance table's intent (which type beats which section) survives a number retune |
| flight/state.rs:302 | `settle_deadband` must stay above the measured doorstep residual |
| flight/autopilot.rs:792 | the urgency denominator moves with the settle band, not independently |

Thirteen `// --- section ---` separators were deleted - 7 in `flight.rs`, 3 in
`gravity.rs`, and 1 each in `audio.rs`, `camera_controller.rs` and `juice.rs`;
`grep -rnE '^\s*//\s*-{2,}'` returns zero over the scope. The one in flight.rs
that headed the integration tests became `flight/tests/mod.rs`'s module doc,
which is what it was actually describing.

No comment that explains WHY was deleted; per the epic rubric only narration,
provenance and dead separators go. No commented-out code was found.

Edited comment paragraphs were re-wrapped so the deletions do not leave ragged
lines; paragraphs whose text is word-for-word unchanged kept master's exact
line breaks, so the diff carries no cosmetic re-wrap churn.

## Evidence

- `cargo check --workspace --all-targets` - clean (the two `nova_os_*`
  ambiguous-import warnings predate this branch).
- `cargo fmt --check` - clean.
- `cargo test -p nova_gameplay --lib` per module: flight **75 passed**,
  camera_controller **14 passed**, audio **30 passed**, gravity **18 passed**
  (1 pre-existing `#[ignore]`), juice **21 passed**, settings **8 passed**,
  damage **8 passed**. 0 failed.
- Test count is conserved exactly: 75 + 14 + 30 = 119 `#[test]` fns before the
  three splits, 119 after.
- The split files were produced by concatenating exact line ranges of the
  originals under hand-written module docs and import headers, so every
  executable line is byte-identical to master; the only edited lines are
  visibility keywords, imports, and comments.
- No defect was uncovered, so no backlog task was opened.

## Doc-surface sweep

Splitting the three files invalidated every comment elsewhere that named one
of them by path. Repointed in `nova_gameplay`
(`hud/lock_crosshairs.rs` x2, `juice.rs` x2, `sections/thruster_section.rs`,
`input/reference.rs`, `input/ai/maneuver.rs`, `input/ai/passive.rs`,
`flight/mod.rs`), in `nova_assets` (`scenario/shakedown.rs`) and in the wiki
(`web/src/wiki/dev/project-tour.md`, whose project-tour table pointed at
`src/flight.rs`).

`grep -rn 'flight\.rs\|camera_controller\.rs\|audio\.rs'` over `crates/` and
`web/src` returns nothing. `tasks/` and `LESSONS.md` are exempt as append-only
history.
