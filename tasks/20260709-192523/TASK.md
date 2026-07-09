# Component-lock HUD: section markers, selection highlight, focus meter

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.4.0,hud,spike


Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

Consumer of the screen-indicator widget (no new substrate): small
entity-anchored markers on the locked ship's live sections in a distinct
color, visible only while focused; the fine-locked section gets a highlighted
variant; a focus meter fills while focusing (thin bar in the readout-column
style first - a radial ring needs image/shader tech the UI pass lacks).
Acquire/lock SFX cues ride the existing audio events where wired (the cue
half of superseded 20260708-165703). Depends on: 20260709-192522 (state to
render).

## Steps

- [ ] New consumer `crates/nova_gameplay/src/hud/component_lock.rs` on the
      screen-indicator widget: a layer spawned/despawned with the player ship
      via hud/mod.rs observers (turret_lead pattern); a reconcile system
      keeps one small fixed-size marker (Entity anchor, distinct color -
      red/orange family, ~10 px) per live section of the locked ship, present
      only while focused; markers despawn on unfocus/lock change.
- [ ] Highlight the fine-locked section: the selected marker gets the
      brighter/larger variant (swap BackgroundColor + size); driven from
      `SpaceshipPlayerComponentLock` each frame.
- [ ] Focus meter: a thin bar (readout-column style) as a child of the
      reticle indicator in hud/torpedo_target.rs, fill =
      focus.seconds/FOCUS_TIME, visible while a lock exists and focus is
      incomplete, gone once focused (the markers appearing is the completion
      signal). Radial ring stays future polish (needs image/shader tech).
- [ ] Tests: markers exist only while focused and match the live section
      count; membership follows section death; highlight follows the
      component lock; meter fraction and visibility windows.
- [ ] Extend examples/12_hud_range.rs: after the lock stage, wait past
      FOCUS_TIME and assert the meter filled then vanished, the section
      markers match the target's live section count, and a directly-written
      component lock highlights its marker; after target death assert the
      markers are gone.
- [ ] Verify: cargo fmt, cargo check --workspace, new + touched tests, one
      scripted 12_hud_range run under Xvfb (report skips).
- [ ] Write the arc doc `docs/2026-07-09-component-lock.md` (or dated when
      it lands): the mechanic end to end, per-consumer behavior deltas
      (enumerated, per the retro lesson), tuning constants with their
      values, and what was deliberately deferred (ring visual, AI component
      picks, faction-based hostility).

## Notes

- Depends on: 20260709-192522 (state to render); the meter also reads
  focus state, so it cannot land before it.
- Marker color must not collide with the amber lead pip or nav cyan; the
  arc doc records the final palette.
