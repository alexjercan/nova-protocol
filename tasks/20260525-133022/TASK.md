# HUD indicator when torpedo is fired

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0,torpedo

Show target lock and torpedo state. Legacy #146.

Pulled into v0.4.0 (roadmap spike 20260708-161726): finishes the torpedo UX that
0.4.0 already polished. There is an existing TODO in
`crates/nova_gameplay/src/hud/torpedo_target.rs` to size the reticle to the target
and add range/lead info - fold that in here.

## Scope (from the user, 2026-07-08)

Two concrete items in this task; broader HUD-for-weapons ideas go into a follow-up
/spike (see the final step).

1. **Crosshair matches target size.** The targeting reticle/crosshair should scale
   to the on-screen size of the object being targeted. It has a MINIMUM size equal
   to its current static width/height, so it can only grow relative to today, never
   shrink below the current look. Project the target's bounds (radius) to viewport
   space to pick the size, clamped to the min.

2. **Easier targeting (autolock).** Raycast-only targeting is too hard to aim.
   Improve it with some form of autolock:
   - A targeting range/cone around the view center: candidate targets within that
     angular threshold are eligible, instead of needing a pixel-perfect ray hit.
   - Snap/lock to the best candidate quickly once it is within the threshold
     (faster snap than today).
   - Cycle between candidate targets by panning the view - as the view moves, the
     lock hands off to the next-best object under a threshold. Keep the current
     panning-to-aim feel, just snappier and with the angular tolerance.

Keep it wasm-safe. This is HUD + targeting UX for the torpedo (and reusable for the
turret target reticle where it makes sense).

## Follow-up (do LAST, after the two items land)

Run a /spike on further "HUD for weapons" UX/gameplay improvements (lead
indicators, lock-on animation/audio cue, range/closing-speed readout, multi-target
tracking, threat direction, ammo/state display, etc.). Capture ideas + a
recommended direction as a docs/spikes/ doc and seed tasks - do not implement them
in this task.

## Resolution (CLOSED - 2026-07-08)

Both items shipped (branch torpedo-hud-133022, reviewed to APPROVE, see REVIEW.md).

1. Reticle sizing: `update_position_indicator_hud` sizes the reticle to the locked
   target's on-screen extent from the union of its collider AABBs
   (`target_world_aabb`), via a bounding-radius projection, clamped to a
   `MIN_RETICLE_PX` (32px) floor - grows only, never shrinks below today's look.
2. Aim-assist: `update_spaceship_target_input` replaces the single sphere-cast with
   angular cone selection (`pick_target`, unit-tested) over dynamic bodies within
   `TARGETING_MAX_RANGE` / `TARGETING_CONE_HALF_ANGLE_DEG`, excluding the player
   ship, static sensor areas, un-committed torpedoes and turret bullets. Point
   roughly and it locks; pan to cycle.

Follow-up still OPEN: the "HUD for weapons" /spike (final step above) is the next
action in this flow.
