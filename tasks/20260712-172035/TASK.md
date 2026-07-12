# Tune PDC turret damage: stop one-shotting asteroids/objects

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0,weapons,balance,playtest

## Goal

Playtest feedback (2026-07-12): the PDC (the player's `better_turret`) destroys
asteroids and objects "with one bullet". Traced: `better_turret` fires
~20.25 authored Kinetic per hit at 100 rounds/s (~2025 DPS); field asteroids are
100 HP, so a burst vaporizes one in ~50 ms - reads as instant. Retune the PDC to a
genuine point-defense profile (low per-hit, high rate) so it chips targets down
over a visible burst instead of popping them, without breaking the shakedown's
"pirate dies in a short burst" intent.

"Done" = the player PDC no longer near-instantly deletes a 100-HP asteroid (a
100-HP target takes a clearly sustained stream, not a blink), catalog data only,
verified by a guard test.

## Steps

- [ ] Extract the better turret's authored damage to a named const in
  nova_assets/sections.rs (e.g. `BETTER_TURRET_BULLET_DAMAGE`) and lower it from
  `representative_kinetic_damage(0.1, 100.0)` (~20.25) to `4.0` - the light
  turret's per-hit, but at 4x the fire rate, so the PDC stays clearly the
  stronger gun (DPS ~400 vs ~95) while a 100-HP asteroid now takes ~25 rounds
  (~0.25 s) instead of ~5. Comment it as a playtest knob with the math.
- [ ] Add a guard test (nova_assets) that `BETTER_TURRET_BULLET_DAMAGE <= 8.0`
  (so a 100-HP object takes >= ~13 PDC rounds - not a one-shot/near-instant pop);
  this fails at the old ~20.25, so it pins the fix intent while leaving tuning
  headroom.
- [ ] Verify `cargo check --workspace --all-targets` + the new test. Update the
  CHANGELOG Unreleased line.

## Notes

- Player ship uses `better_turret_section` (nova_assets/scenario.rs:285,345), so
  that is the PDC to retune. `light_turret` (scavenger, 3.825/hit @ 25 rps) is
  already gentle - leave it.
- Data-only balance change; no code path changes. The typed-damage core is
  unaffected (still Kinetic x resistance). Asteroids are unclassed (resistance
  1.0), so the per-hit reduction is the whole lever.
- Tradeoff: this also slows ship-vs-ship TTK ~5x for the player PDC. That is
  consistent with a point-defense weapon and with the shakedown pirate still
  dying in a short burst (60-HP hull -> ~15 rounds -> ~0.15 s). If playtest wants
  it punchier vs ships, raise the const (the guard allows up to 8.0).
- Alternative considered (not taken): buff asteroid HP instead. Rejected - the
  user's report is "PDC damage too high", and lowering the gun is the smaller,
  reversible change that also fixes the too-fast ship kills.
