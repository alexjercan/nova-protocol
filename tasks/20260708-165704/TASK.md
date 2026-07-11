# Off-screen target/threat edge indicators (HUD)

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.5.0, hud, spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 3. When a locked target or an incoming threat (e.g. a hostile torpedo) is off
the screen, show a direction arrow clamped to the screen edge pointing at it, so the
player knows where to turn. Consumer of the screen-projected-indicator widget
(20260708-165700), which should own the off-screen edge-clamping logic.

"Threat" needs a notion of what is dangerous to the player (incoming torpedoes,
hostiles); scope that during planning - a first cut can just point at the current
lock and at committed enemy torpedoes.

Note (20260711): the multi-target spike
docs/spikes/20260711-163800-multi-target-cycle.md makes the candidate set built
for 20260708-165705 the intended data source here - edge indicators point at
off-screen candidates, the active lock, and committed hostile torpedoes
(relation Hostile via relations.rs).

## Goal

Off-screen tracked entities - the active lock, candidate hostile ships, and
committed hostile torpedoes - each show an arrow clamped to the screen edge
pointing at them; on-screen entities show nothing extra. First consumer of
the widget's `ClampToEdge` + `ScreenIndicatorArrowMarker` path.

## Steps

- [ ] Arrow art: a small up-pointing chevron/triangle as UI content (the
      widget rotates it via `UiTransform`, expecting up-pointing art - see
      `ScreenIndicatorArrowMarker` in hud/screen_indicator.rs). No existing
      consumer or asset: build it from UI nodes (e.g. two angled bars or a
      bordered rotated square) or a tiny embedded image, matching how the HUD
      sources its other art. Keep it a reusable `edge_arrow()` bundle fn.
- [ ] New `crates/nova_gameplay/src/hud/edge_indicators.rs`: layer marker +
      `edge_indicators_hud()` root (a `screen_indicator_layer()`), registered
      in `hud/mod.rs`'s NovaHudPlugin and the player HUD spawn/despawn
      observers like the other overlays.
- [ ] Tracked-set reconcile system (pattern: `sync_component_markers` in
      hud/component_lock.rs): one indicator per tracked entity, where the
      tracked set = `SpaceshipPlayerTargetLock` + entries of
      `SpaceshipPlayerTargetCandidates` (from 20260708-165705) + committed
      hostile torpedoes (`TorpedoProjectileMarker` + `TorpedoTargetChosen` +
      `relation(player, torpedo) == Relation::Hostile`, same query shape as
      update_spaceship_target_input's). Each indicator:
      `ScreenIndicatorOffscreen::ClampToEdge { margin_px: ~24 }`, small Fixed
      size, and the arrow as its only visible content - the widget hides the
      arrow while the anchor is on-screen, so on-screen entities render
      nothing (verified: update_arrows sets Hidden when not clamped).
- [ ] Styling by kind: active lock arrow bright (reticle hostile red /
      untinted per relation, match torpedo_target.rs colors), hostile torpedo
      arrows full red, candidate arrows dim - restyle in the reconcile or a
      chained style system like `highlight_selected_marker`.
- [ ] Tests mirroring component_lock.rs's: membership follows lock change,
      candidate churn, torpedo commit/death; a `place`-level assertion is NOT
      needed (widget already tested) - test only the reconcile + styling.
- [ ] Full check suite; append what shipped to the weapons-hud spike doc
      (docs/spikes/20260708-165647-weapons-hud.md) Fix record.

## Notes

- Depends on: 20260708-165705 (SpaceshipPlayerTargetCandidates resource).
- Key files: crates/nova_gameplay/src/hud/screen_indicator.rs (ClampToEdge,
  arrow rotation, behind-camera handling all already implemented and tested),
  crates/nova_gameplay/src/hud/component_lock.rs (reconcile pattern),
  crates/nova_gameplay/src/hud/torpedo_target.rs (relation tint colors),
  crates/nova_gameplay/src/relations.rs.
- The behind-camera case is handled by the widget (virtual point clamped to
  the correct edge) - no extra work here.
- Avoid double-marking: candidates already get on-screen brackets from 165705
  (offscreen Hide) and the lock gets the reticle; this overlay adds ONLY the
  arrow content, which self-hides on-screen, so no visibility coordination is
  needed between the overlays.
