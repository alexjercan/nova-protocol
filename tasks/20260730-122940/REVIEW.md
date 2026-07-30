# Review: Wide keycaps render at their art aspect, height-constrained

- TASK: 20260730-122940
- BRANCH: fix/keycap-aspect

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/keybind_dock.rs:1889 -
  `assert!(glyphs.len() > 0, ...)` trips `clippy::len_zero` (`KeyGlyphs` has an
  inherent `is_empty`). Change it to
  `assert!(!glyphs.is_empty(), "delivery guard: the rig loaded keycaps")`.
  - Response: Done. Confirmed `KeyGlyphs::is_empty` exists and the guard reads
    the same.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/key_glyphs.rs:186 - `trimmed_cap`
  is `pub` but not re-exported from `key_glyphs::prelude`, and AGENTS.md says
  new public items go through the prelude; either add it or narrow it to
  `pub(crate)`.
  - Response: Added to the prelude rather than narrowed - backlog 20260728-214929
    adopts these glyphs on surfaces outside this crate and will want the same
    trim.

The reviewer also noted, as prose rather than a finding, that the `measure_caps`
warn's denominator counts LABELS (18) while the scan dedups to 13 distinct
images - correct as a "did everything resolve" check but loose in wording. Taken:
the warn and `KeyGlyphs::len`'s doc now say labels and explain the sharing.

### Proofs run by the reviewer

- DoD 1: `cargo test -p nova_gameplay --lib keycap_sizing` - 4 passed.
- DoD 2: re-ran `rg -n 'GLYPH_PX|CUE_GLYPH_PX' crates/nova_gameplay/src/hud`
  independently; three production uses remain, each a HEIGHT passed into
  `KeyCap::apply`/`KeyCap::node`, no site sets `width == height`.
- DoD 3: `every_preloaded_glyph_resolves_a_plausible_cap` walks all of
  `KEY_GLYPH_FILES` and requires `resolved == len` with each cap bounded inside
  the canvas at >= 64 px per axis.
- DoD 4: `DISPLAY=:99 cargo test --test examples_smoke screenshots` - 1 passed.
- Plus `--lib tab_footer_sizing` (1 passed), `--lib hud::` (296 passed, no
  existing HUD test weakened), `cargo test -p nova_assets --lib key_glyph`, and
  `cargo fmt --check`.
- Eyeballed `shots/dock-before.png` vs `shots/dock-after.png` and confirmed the
  crop and scale match and the legibility claim holds.

The reviewer checked the new tests for tautology specifically and reported them
sound: the expectations are hand-transcribed `magick` bounds, the layout rigs
compare a real `ComputedNode` against `<site const> * measured_w/measured_h`, and
every tolerance is tight enough that the old square box fails (CTRL 33.3 vs 22,
X 20.3 vs 22, cue O 18.5 vs 20, footer Tab 27.2 vs 18), with
`ImageNode.rect == Some(cap)` failing on top.

### Pending user checks (open `manual:` DoD)

- DoD 5: "Owner can read TAB / SHIFT / CTRL at a glance in game." The after-shot
  is supporting evidence; the in-game read is the owner's call at Finish.

### In-session re-derivation

Per the review skill, the in-session pass re-derived load-bearing claims rather
than adopting the round wholesale: both NIT findings were confirmed against the
source before fixing (`KeyGlyphs::is_empty` exists at key_glyphs.rs:258; the
prelude at key_glyphs.rs:33 did omit `trimmed_cap`), and the DoD 2 grep was run
in-session as well as by the reviewer. `keycap_sizing` re-run green after the
fixes.
