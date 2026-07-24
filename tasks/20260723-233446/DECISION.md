# DECISION: allegiance marker - artifact, scope, look

- DATE: 20260724
- TASK: 20260723-233446
- STATUS: ACCEPTED

## Context

The task ("small triangle/chevron above each ship, coloured by allegiance")
fixed the placement and intent but left three load-bearing forks open in its own
Notes: which ships get a marker, whether the mechanism is a world billboard vs a
radar chip, and the exact look. These change the DoD, the tests, and the on-
screen feel, so they were settled with the user before any code was cut.

## Decision

1. **Concrete artifact: a `screen_indicator`-based HUD marker, one per ship.**
   New submodule `hud/allegiance_markers.rs` built on the `beacon_chips.rs`
   template: an `Add<SpaceshipRootMarker>` / `Remove<SpaceshipRootMarker>`
   observer pair spawns/despawns one `screen_indicator` layer per ship, anchored
   to the ship entity and offset upward so the marker floats above the hull.
   Rejected: a 3D billboard mesh (a second render path, heavier, and the whole
   HUD already projects world anchors to UI nodes) and a radar-only chip (the
   task explicitly wants it above the hull in the world view).

2. **Scope: all ships incl. Neutral, skip the player's own ship.**
   Mark every entity carrying `SpaceshipRootMarker` EXCEPT the one carrying
   `PlayerSpaceshipMarker`. Colours from `nova_ui::theme::semantic`:
   Player -> `ALLY` (green), Enemy -> `THREAT` (red), Neutral / no allegiance ->
   `NEUTRAL` (grey). Neutral bystanders are shown (grey) for consistency; the
   player's own ship is skipped to keep screen-centre clean (you know where you
   are). AI wingmen (`Allegiance::Player`) still get a green marker.

3. **Look: filled down-pointing triangle, `Offscreen::Hide`.**
   A small FILLED triangle pointing down at the hull (user's choice over the
   outline chevron), drawn with the CSS-border-triangle trick (zero-size node,
   coloured top border, transparent left/right) tinted by the allegiance colour
   - no art asset. Off-screen policy is `Hide` (not edge-clamp): the marker is a
   glance cue for on-screen ships; off-screen pointing is already the edge
   indicators' job. Tier `HudTier::Instrument` (survives Minimal, clears at
   cinematic None).

## Consequences

- The feature is one cohesive task (new module + mod.rs wiring + observers +
  recolour system + tests + docs), landed as a single commit.
- Cost is bounded: one fixed-size UI node per on-screen non-player ship, hidden
  off-screen, projected by the existing `ScreenIndicatorSystems` pass. Verified
  by a probe run against a mixed-allegiance combat example (no fps regression).
- Runtime allegiance flips (`SetAllegiance`) are honoured by a recolour system,
  so neutral-until-provoked haulers turn red when they turn hostile.
</content>
