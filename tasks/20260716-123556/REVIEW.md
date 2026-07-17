# Review: reload-state on the diegetic ammo readout

- TASK: 20260716-123556
- BRANCH: feature/ammo-readout-reload-state

## Round 1

- VERDICT: APPROVE

Verified independently (implementer == reviewer, so the load-bearing claims were
re-derived, not read):

- **The "RELOAD_ALPHA below threshold" regression claim holds.** The shipped
  `lit_pip_count` counts pips with `alpha > (LIT_ALPHA + DIM_ALPHA)/2 = 0.555`.
  `RELOAD_ALPHA = 0.5 < 0.555`, so reload pips are never miscounted as live
  rounds, and `> 0.16` so they aren't "dim" either. The shipped drive tests
  attach no `SectionReload`, so their rendering is untouched - and the new
  `driver_at_rest_reload_is_identical_to_no_reload` pins that a `SectionReload`
  present-but-not-reloading changes nothing. The three shipped drive tests are
  present and unmodified (grep confirmed).
- **Sweep math is bounded and correct.** `reload_fill_segments` computes
  `remaining = seg - steady` (saturating) and `fill = round(clamp(progress) *
  remaining)`, so `reload_end = steady + fill <= steady + remaining = seg` -
  it can never exceed the pip count. Hand-checked: turret empty @0.5 ->
  `round(4.0)=4`; torpedo 1/4 @0.5 -> `round(1.5)=2`. Both match the tests.
- **Genuinely one path across both modes.** `reload_end` derives from
  `is_reloading()` + `progress()` + `reload_fill_segments`, with no branch on
  `only_when_empty`; the mode is encapsulated in `is_reloading`. Discrete (empty,
  `steady = 0`) fills the whole ring; continuous (`steady = rounds`) lights the
  rounds coming back above the live ones. Correct for the turret ring and the
  torpedo bar alike.
- **Tests can fail with the fix removed.** The turret test's A/B leg removes the
  `SectionReload` and asserts the sweep drops to zero - so the sweep, not some
  other lighting, is what it measures. The pure-math test covers clamp and the
  full-gauge (nothing to sweep) edge.
- **Docs in sync.** Module doc, CHANGELOG (Interface & HUD), NOTES.md, spike Fix
  record, and a new player-facing "Ammo & reloading" section in
  `combat-weapons.md` (which also backfills the finite-ammo/auto-reload
  description the mechanic task left out of the player wiki - a good catch, not a
  scope creep). `cargo fmt --check` clean; `hud::ammo_readout` 13/13.

No BLOCKER/MAJOR/MINOR findings. The design is a minimal, additive change to the
single ammo-state read point, exactly what the spike's option B1 asked for, with
the corner-chip (B2) correctly not built.
