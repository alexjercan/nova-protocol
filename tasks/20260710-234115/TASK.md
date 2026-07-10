# Engaged-state shader tint across the flight instrument family

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0,hud,ux


## Goal

Make engaged-vs-manual readable at a glance from the instruments
themselves, reinforcing the ship-anchored mode chip (task
20260710-231926) diegetically, per the user's "chip + shader tint"
questionnaire choice.

Scope refinement at /plan time (2026-07-11): the ribbon, orbit ring,
flip gate, and radius spoke only EXIST while a maneuver is engaged -
they cannot carry an engaged-vs-manual signal and are already uniformly
NAV_CYAN, the flight computer's family color. The only always-visible
family member is the velocity sphere, so the tint is: the velocity
widget's palette shifts to the nav-cyan family while the player's
autopilot is engaged and reverts to the white/blue default in manual.
The gravity sphere stays yellow in both states - it reports the world,
not who is flying.

## Steps

- [x] Add `VelocityHudPalette::ENGAGED` in
      crates/nova_gameplay/src/hud/velocity.rs - nav-cyan family
      (indicator fully opaque NAV_CYAN-ish, sphere low-alpha cyan tint,
      values tuned by eye later); derive PartialEq on VelocityHudPalette
      so the current palette can be compared to a target palette.
- [x] Add a `sync_engaged_palette` system in velocity.rs: for widgets
      with `VelocityHudSource::Velocity`, desired palette = ENGAGED when
      the target entity has `Autopilot`, default otherwise; when it
      differs from the widget's current `VelocityHudPalette` component,
      update the component and write both child materials' base_color
      (indicator: DirectionMagnitudeMaterial ext, sphere:
      DirectionSphereMaterial ext - mutate via Assets get_mut like
      direction_shader_update_system). Guard on inequality so material
      assets are not dirtied every frame. Gravity-source widgets are
      skipped entirely.
- [x] Register the system in VelocityHudPlugin's Update tuple
      (NovaHudSystems set).
- [x] Unit tests in velocity.rs: palette component flips to ENGAGED on
      engage and back on disengage; a gravity-source widget's palette
      never changes; spawn-state honesty - a ship that spawns WITH an
      engaged autopilot gets the ENGAGED palette on the system's first
      run (materials are children; asset mutation is covered by
      inspection of the component seam if child materials are awkward
      headless - the system should split a pure "desired palette"
      helper for the decision logic).
- [ ] By-eye pass (piggyback, from retro 20260710-231926): the chip
      offsets (120px right of ship, mode row above), spoke thickness,
      and the new tint colors - run the game, orbit something, engage
      and disengage; adjust constants.
- [x] cargo fmt + cargo check --workspace --examples; run the velocity.rs
      tests.
- [x] CHANGELOG.md [Unreleased], Changed: velocity sphere tints to the
      nav-cyan family while the autopilot flies, reverting in manual.

## Notes

- Spike: docs/spikes/20260710-234019-diegetic-flight-status.md
- Palette colors are baked into the two child materials at spawn by the
  On<Add> observers; each widget owns unique material instances (one
  `materials.add` per child), so mutating them tints only that widget.
- The spawn-time palette stays the config's palette (default white/blue
  for the velocity variant): sync_engaged_palette converges it on first
  run, and the retro lesson about spawn-state-from-the-same-predicate is
  satisfied by asserting that first-run convergence in a test.
- Blend/fade between palettes was considered and deferred: an instant
  swap is one guarded write; a fade dirties the material asset every
  frame of the transition. If the swap feels harsh in the by-eye pass,
  file a follow-up.

## Closing notes (2026-07-11)

What changed: VelocityHudPalette gained PartialEq, an ENGAGED nav-cyan
const, and the pure desired_velocity_palette helper; sync_engaged_palette
flips the palette component and rewrites the two child materials'
base_color only on a state change (guarded compare, so material assets
are not re-uploaded per frame). Gravity-source widgets are skipped by
source, not by palette value. Tests cover the pure helper, first-run
convergence for a ship that spawns already engaged, disengage reverting,
and the gravity widget staying yellow.

The by-eye step is deliberately unticked: this session is headless, so
the ENGAGED color values (NAV_CYAN rgb at the default alphas) and the
earlier chip offsets/spoke thickness still need a human playtest; the
constants are all named and documented for that pass.

Difficulties: one compile fix - Assets::get_mut returns a guard that
needs a mut binding, not a plain &mut. Reflection: the plan-time scope
refinement (holos are engaged-only, so the sphere is the tint's whole
payload) turned a vague "family-wide shader treatment" into a one-system
change; reading the rendering seam before planning is what caught it.
