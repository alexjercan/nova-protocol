# HUD: friendly/enemy allegiance marker over ships (small triangle/chevron above each entity)

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.9.0, hud, gameplay

## Story

Playtest of the ch5 raid (task 20260723-182855) surfaced this: with two AI
wingmen on the player's side and four enemy fighters in the same brawl, it is
hard to tell friend from foe at a glance. Add a small world-anchored marker
(a filled triangle above each ship) coloured by allegiance so the player can
read the fight instantly.

## Decisions (see DECISION.md)

- Concrete artifact: a new HUD submodule `allegiance_markers.rs` that spawns one
  `screen_indicator`-based marker per ship, built on the same observer template
  as `beacon_chips.rs` (Add/Remove on the ship marker). NOT a 3D billboard mesh
  and NOT a radar-only chip.
- Scope: mark ALL ships including Neutral, but SKIP the player's own ship
  (`PlayerSpaceshipMarker`). Colours from the semantic theme:
  Player -> `ALLY` (green), Enemy -> `THREAT` (red), Neutral / no allegiance ->
  `NEUTRAL` (grey).
- Look: a small FILLED triangle pointing down at the hull, drawn with the
  CSS-border-triangle trick (zero-size node, coloured top border, transparent
  left/right) tinted by the allegiance colour - no art asset.
- Off-screen: `ScreenIndicatorOffscreen::Hide` (no edge-clamp; the marker is a
  glance cue for on-screen ships, not an off-screen pointer - the edge
  indicators already own that). Hides on death because death despawns the ship
  (Remove observer). Tier: `HudTier::Instrument` (combat-relevant; survives
  Minimal, clears at cinematic None).

## Steps

1. New submodule `crates/nova_gameplay/src/hud/allegiance_markers.rs`:
   - `AllegianceMarkerHudPlugin`, `AllegianceMarkerHudMarker`,
     `AllegianceMarkerTargetEntity(Entity)`, and a `prelude`.
   - `allegiance_color(Option<&Allegiance>) -> Color` mapping (Player/Enemy/
     Neutral/None). Pure, unit-testable.
   - `allegiance_marker_hud(ship, color)` bundle: `screen_indicator_layer` +
     child `screen_indicator` (Entity anchor on `ship`, `ScreenIndicatorSize::
     Fixed` small, an upward `offset` so the triangle floats above the hull,
     `Offscreen::Hide`) whose content is the filled down-triangle node.
2. Wire it in `hud/mod.rs`: add the submodule + prelude re-export, add the
   plugin, register types.
3. Lifecycle observers (beacon_chips pattern):
   - `Add<SpaceshipRootMarker>` -> spawn a marker unless the ship is the player
     (`PlayerSpaceshipMarker`); colour from the ship's `Allegiance` at spawn.
   - `Remove<SpaceshipRootMarker>` -> despawn the ship's marker (hide on death).
4. Runtime recolour: a small system (or `Add<Allegiance>`/change detection) that
   updates the triangle colour when a ship's `Allegiance` flips at runtime (the
   `SetAllegiance` scenario action, neutral-until-provoked). Keep it in
   `NovaHudSystems`.
5. Harness/unit tests in the module (see DoD).
6. Docs: HUD wiki page(s) under `web/src/wiki/` note the new allegiance marker
   (player-facing HUD change); CHANGELOG line.

## Definition of Done

- Each non-player ship shows an allegiance-coloured filled triangle above its
  hull; friendly (green) vs enemy (red) vs neutral (grey) is readable at a
  glance; the marker follows the ship, hides off-screen, and disappears on
  death. The player's own ship shows none.
  (manual: fly the ch5 raid / a mixed-allegiance scenario and confirm friend vs
  foe reads instantly; own ship has no marker.)
- test: a headless HUD test (App-driven, production-faithful scheduling) that
  spawns ships of each allegiance + the player ship, runs the real observers,
  and asserts: one marker per NON-player ship, correct colour per allegiance,
  the player ship gets none, a despawned ship's marker is gone, and a runtime
  `SetAllegiance`-style flip recolours the triangle.
- test: `allegiance_color` unit test covering Player/Enemy/Neutral/None.
- cmd: `cargo run -p nova_probe -- run <a combat example with mixed allegiances>`
  shows no measurable fps regression vs before (it draws every ship every
  frame - see perf task 20260723-233453).

## Notes / pointers

- Allegiance enum: `crates/nova_gameplay/src/relations.rs`.
- World-to-screen anchoring: `hud/screen_indicator.rs` (the reusable widget);
  `hud/beacon_chips.rs` is the one-indicator-per-entity observer template.
- Semantic colours: `nova_ui::theme::semantic` (`ALLY`, `THREAT`, `NEUTRAL`).
- Ship root marker: `SpaceshipRootMarker` (`sections/mod.rs`); player ship also
  carries `PlayerSpaceshipMarker`. Wingmen are AI ships with
  `Allegiance::Player` and still get a green marker.
- Keep it cheap: fixed-size node, `Offscreen::Hide`, no per-frame allocation.
</content>
</invoke>
