# Comments and docs - the measurement

**Read this before proposing any comment rule.** The task's original premise was
"there are useless comments all over the code". It was measured by two
independent agents and **does not hold**. The real problem is different.

## Inventory

`crates/`, 384 `.rs` files, 155,587 lines.

| Crate | LOC | `//` | `///` | `//!` | `/* */` |
| --- | --- | --- | --- | --- | --- |
| nova_gameplay | 77,761 | 5,150 | 8,585 | 1,823 | 0 |
| nova_assets | 27,476 | 1,708 | 2,461 | 859 | 2 |
| nova_scenario | 15,127 | 844 | 2,022 | 167 | 1 |
| nova_probe | 10,328 | 396 | 910 | 506 | 3 |
| nova_menu | 8,154 | 360 | 795 | 47 | 0 |
| nova_editor | 2,378 | 120 | 139 | 43 | 0 |
| nova_os | 2,560 | 111 | 400 | 58 | 0 |
| nova_autopilot | 3,883 | 111 | 585 | 453 | 0 |
| nova_ui | 3,703 | 84 | 590 | 174 | 0 |
| nova_debug | 1,643 | 77 | 349 | 128 | 0 |
| nova_core | 689 | 36 | 90 | 48 | 0 |
| nova_events | 821 | 19 | 185 | 20 | 0 |
| nova_modding | 439 | 15 | 99 | 31 | 1 |
| nova_mod_format | 531 | 10 | 131 | 24 | 0 |
| nova_events_macros | 59 | 6 | 5 | 8 | 0 |
| nova_info | 35 | 0 | 4 | 4 | 0 |
| **Total** | **155,587** | **9,047** | **17,350** | **4,393** | **7** |

All 7 `/* */` are inside doc examples or string literals.

`//` comments form 3,917 contiguous blocks - **2.31 lines per block**. The
median comment is a short paragraph, not a one-line label. 1,446 of 9,047 (16%)
are in test files.

## Classification - 70-block random sample

| Category | Count | % |
| --- | --- | --- |
| (c) **WHY / constraint / guard** | 58 | **83%** |
| (a) restates adjacent code | 8 | 11% |
| (b) header / divider | 4 | 6% |
| (d) TODO/FIXME/HACK | 0 | 0% |
| (e) commented-out code | 0 | 0% |

Whole-corpus proxy confirms it: 389 single-line blocks with text under 45 chars
(the restatement-prone shape) plus 50 ruled dividers = **439 of 9,047 = 4.9%**.

### (c) exemplars - the style to preserve

- `nova_gameplay/src/hud/screen_indicator.rs:459` - "This runs BEFORE this
  frame's transform propagation (UI layout comes...)"
- `nova_gameplay/src/hud/lock_crosshairs.rs:624` - "Registered ONCE so the
  MessageReader cursor persists across runs"
- `nova_gameplay/src/sections/torpedo_section/render.rs:380` - "Low graphics
  tier is spawn-less: skip the launch-burst hanabi."
- `nova_gameplay/src/audio/sfx.rs:124` - "NOTE: rodio does not accept a
  non-positive playback rate."
- `nova_probe/src/capture.rs:457` - reload frames excluded, and why the next
  frame too
- `nova_probe/src/recorder.rs:213` - `File::create` truncation hazard
- `nova_gameplay/src/asset_ref.rs:24-28` - why `Clone`/`Debug` are hand-written
  rather than derived (a derive would add a wrong `A: Clone` bound)

### (a) exemplars - genuine noise

- `nova_gameplay/src/damage.rs:151` - "// Concussive: red-orange fire." over
  `DamageType::Explosive => Color::srgb(1.0, 0.4, 0.15)`
- `nova_gameplay/src/juice.rs:730` - "// Monotonic decreasing." over
  `assert!(flash_alpha(0.25) > flash_alpha(0.75))`
- `nova_menu/src/tests/mods.rs:43` - "// Disable." over `trigger(Activate...)`
- `nova_gameplay/src/hud/mod.rs:1092` - "// Visible in normal flight." over
  `app.update()`

Also: `camera/rig.rs:176`, `hud/component_lock.rs:138`, `hud/turret_lead.rs:152`,
`hud/ammo_readout.rs:359`, `hud/readout.rs:176`, `nova_os_map/tests.rs:390`.

### (b) exemplars

`nova_assets/tests/gen_portal_gate.rs:127` (a bare `// ---...---` rule),
`nova_assets/tests/ledger_ch4_ending.rs:448`, `nova_gameplay/src/plugin.rs:93`
("// Sphere Orbit Plugin"), `nova_gameplay/src/sections/thruster_section.rs:677`
("// Assert"). **All 36 ruled dividers are in `nova_gameplay/src/hud/nova_os*`.**

## Worst offenders - the ranking is flat

(a)+(b)+(e) as a share of `//` comments:

nova_gameplay 5.7% (292 of 5,150) - nova_os 5.4% - nova_probe 5.1% -
nova_editor 5.0% - nova_scenario 4.0% - nova_assets 3.9% - nova_core 2.8% -
nova_debug 2.6% - nova_ui 2.4% - nova_menu 2.2% - nova_autopilot 1.8% -
events/modding/mod_format/macros ~0%.

**No crate is an outlier.** nova_gameplay leads on absolute volume only (292 of
the 439 total).

## Doc comments

All 16 crates carry `#![warn(missing_docs)]` in `src/lib.rs`. The only escape
hatch is `#[allow(missing_docs)]` at `nova_assets/src/portal/mod.rs:108`.

Docs are substantive, not signature echoes: 5,734 `///` blocks over 17,350
lines = **3.03 lines per block**, 3,667 (64%) multi-line.

Sampled field docs in `nova_gameplay/src/flight/state.rs` carry units, ranges
and rationale - "Analog main-drive burn, `0..1` (W / Space / right trigger)",
"Speed along the line to the goal, u/s (negative = opening)", "An estimate for
the instruments, not a promise". `shader_sample_uv_reference` in
`nova_os_pointer_rig.rs` documents why it must not reuse the production helper.

The `/// The ship's health` on `health: f32` failure mode is **rare here**.

## Markers and dead code

- **3 TODO markers workspace-wide** (0.02 per 1k LOC). No FIXME, HACK or XXX
  anywhere.
  - `nova_assets/src/collections.rs:236` - `// TODO(20260525-133028): Probably
    need to refactor this somehow`
  - `nova_gameplay/src/hud/key_glyphs.rs:25` - remapping/gamepad
    `TODO(20260710-231927)`
  - `nova_gameplay/src/input/reference.rs:10` - same tracker id
  - Two share one id, so there are really two open items.
- **Zero commented-out code.** A scan returned 28 candidate hits; 26 were prose
  continuation lines. The only real ones -
  `nova_gameplay/src/hud/nova_os_pointer_rig.rs:64` and `:67` - are intentional
  WGSL source quoted beside the Rust reimplementation of the shader.

## Deletion estimate

- (a)+(b)+(e) corpus proxy: 389 short single-line blocks + 50 ruled dividers +
  0 dead code = **439 lines**
- Sample-rate cross-check: 12 of 70 blocks flagged -> ~666 blocks, all
  one-liners -> ~666 lines. Test-case labels are about half and are defensible,
  giving **350-450**
- Aggressive variant, including tightening multi-line prose: ~1,300 lines (14%
  of comments), but that is rewriting, not deleting

**Point estimate: 430-670 lines, about 0.3-0.4% of total LOC.**

## Conclusion - the premise was wrong, but something else is right

A blanket comment purge would delete a large amount of genuine constraint
documentation for a ~0.3% line win. **Do not do it.**

The real problem, found independently by two agents, is **volume and
staleness**:

| Symptom | Evidence |
| --- | --- |
| Task-artifact references that will rot | `nova_gameplay/src/hud/nova_os/mod.rs:18` "see this task's DECISION.md"; `nova_os_ship/mod.rs:33` "DECISION fork 4"; `nova_assets/src/portal/mod.rs:3` task id "142906"; `nova_assets/Cargo.toml:15` |
| Self-congratulatory history | `nova_ui/src/theme.rs:130-138` |
| Duplicated manuals | `nova_probe/src/lib.rs` - **100 comment lines in 168**, duplicating `.claude/skills/probe/SKILL.md`. Every `#[cfg]` there carries a 2-4 line justification paragraph |
| Prose-heavy composition roots | `nova_core/src/lib.rs` - 304 lines, ~135 prose. Root `Cargo.toml` - ~120 lines of profile commentary including measured RSS numbers |
| Repeated boilerplate docs | **91 identical** `/// Glob-import surface: use nova_x::y::prelude::* re-exports the public API of this module.` across 91 files |
| Rot that already happened | AGENTS.md's `nova_modding` row is wrong on 3 of 4 items. See `02-workspace-map.md` |

**Proposed test for CONVENTIONS.md: a comment must survive the next refactor.**
That catches all six rows above and none of the (c) exemplars.

Note that `#![warn(missing_docs)]` is on all 16 crates, so "delete the doc" is
not available for public items. Any rule must be about what the doc *says*.

## Independently corroborated 2026-08-07 - from a different direction

`13-review-cross-cutting.md` swept the workspace for the patterns people
usually assume are wrong in a fast-built codebase, and reached the same
conclusion this note reached about comments: **hygiene here is consistently
better than a first impression suggests.**

| Pattern | Raw hits | Sampled | Genuinely bad |
| --- | --- | --- | --- |
| `as` casts (truncating/wrapping) | 110 | 8 | **0** |
| Float equality | 14 | 6 | **0** |
| Division by possibly-zero | 18 | 8 | 1 |
| `unwrap`/`expect`/`panic!` non-test | 23 real | 12 | 3 |
| Unbounded indexing/slicing | ~80 | 8 | 1 |
| Duplicate system registrations | 42 candidates | all | **0** |
| `pub` items unused anywhere | - | 13 | **0** |

Two methods, two corpora, same answer. That matters for CONVENTIONS.md: a rule
adopted because it "sounds like good Rust" will land on a clean codebase and
produce nothing but false positives. Every candidate rule should arrive with
a measured violation count, and `09-clippy-and-lints.md` bucket 3 supplies one
for each style lint.

The one comment-adjacent finding the sweep added, and this note did not have:

**Convert the `#[allow(clippy::type_complexity)]` suppressions to
`#[expect(..., reason = "...")]`.** VERIFIED 2026-08-07: **35 in `crates/`, 2
in `src/`+`examples/`, 37 workspace-wide** (`13-review-cross-cutting.md` says
36; the workspace total is 37, and 08's figure of 37 is the correct one), against
**4** existing `#[expect(clippy::..., reason = "...")]`
(`input/player/hints.rs:200`, `keybind_dock.rs:569,737,790`). So this enforces
an existing local convention rather than importing a new one, and rustc's
`unfulfilled_lint_expectations` then reports every stale suppression for free.
Two are already known stale by eye (`hud/ammo_readout.rs:325`, `:510`).

This is the same class of problem as a stale comment - a claim about the code
that the code no longer supports - and it is the only one of the class that the
compiler can be made to audit. It pairs with the `-D warnings` recommendation.
