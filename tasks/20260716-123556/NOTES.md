# Notes: reload-state on the diegetic ammo readout

## What shipped

- `drive_ammo_readouts` (`crates/nova_gameplay/src/hud/ammo_readout.rs`) now also
  reads `Query<&SectionReload>` and, while a section is reloading, renders a
  reload sweep over the pips above the live-round level in the loaded hue at a
  new `RELOAD_ALPHA` (0.5).
- `reload_fill_segments(segment_count, steady_lit, progress)` - the pure,
  gauge-agnostic sweep math (how many pips above the steady level the sweep has
  filled), unit-tested independently.
- Module doc, player wiki (`combat-weapons.md` gained an "Ammo & reloading"
  section - the first player-facing description of finite ammo + auto-reload,
  which task 20260717-085640 had only documented in the CHANGELOG and dev wiki),
  CHANGELOG.

## Design

The visual is a pure function of `SectionReload::progress()` and the steady lit
count, so one code path serves both reload modes without branching on the mode:

- discrete turret reload (empty, `only_when_empty`): `steady_lit = 0`, so the
  sweep fills the whole ring from empty to full as progress runs 0->1;
- continuous torpedo regen: `steady_lit = rounds`, so the live rounds stay lit
  and the sweep lights the rounds coming back above them.

Three pip states now: live (`LIT_ALPHA` 0.95), reloading (`RELOAD_ALPHA` 0.5),
dim (`DIM_ALPHA` 0.16). `RELOAD_ALPHA` sits below the existing tests'
lit-vs-dim threshold `(LIT+DIM)/2 = 0.555`, so `lit_pip_count` still counts only
live rounds and the shipped drive tests are unaffected - a reload pip is neither
"lit" nor "dim" to the old helper.

## Why no separate corner chip (spike B2 rejected)

The spike concluded loaded-type + count already ship on this diegetic readout;
the only missing signal was reload state. Adding it here (option B1) keeps one
readout for one weapon instead of duplicating type/count into a second HUD
widget. See tasks/20260716-123556/SPIKE.md.

## Tests

- `reload_fill_segments_fills_the_remaining_track_with_progress` - pure math,
  including clamp and full-gauge (nothing to sweep).
- `driver_sweeps_the_ring_while_a_turret_reloads` - empty + mid-cycle discrete
  reload lights `floor(progress * RING_SEGMENTS)` reload pips and 0 live; A/B
  removing the `SectionReload` drops the sweep to nothing (proves the sweep, not
  some other lighting, is responsible).
- `driver_sweeps_the_torpedo_bar_above_the_live_rounds_while_rearming` - one live
  round stays lit, the rearming rounds show in reload hue above it.
- `driver_at_rest_reload_is_identical_to_no_reload` - a full mag carrying a
  `SectionReload` is not reloading, so rendering is byte-identical to the shipped
  steady path (regression guard for loaded-type/count).
- The existing drive tests (`driver_lights_turret_chunks_by_fraction` etc.) pass
  unchanged, confirming no regression when no reload is in flight.

## Self-reflection

Choosing `RELOAD_ALPHA` below the existing test threshold was deliberate - it let
the new state coexist with the shipped `lit_pip_count`/`first_lit_pip_color`
helpers without touching them, so the regression guard is honest. The one thing
to watch on a future visual pass: `RELOAD_ALPHA` is a flat mid-alpha, not an
animated pulse; if playtest wants motion, the hook is `progress()` plus a time
source, a local change to this one system.
