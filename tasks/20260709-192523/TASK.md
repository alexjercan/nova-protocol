# Component-lock HUD: section markers, selection highlight, focus meter

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.4.0, hud, spike

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

- [x] New consumer `crates/nova_gameplay/src/hud/component_lock.rs` on the
      screen-indicator widget: a layer spawned/despawned with the player ship
      via hud/mod.rs observers (turret_lead pattern); a reconcile system
      keeps one small fixed-size marker (Entity anchor, distinct color -
      red/orange family, ~10 px) per live section of the locked ship, present
      only while focused; markers despawn on unfocus/lock change.
- [x] Highlight the fine-locked section: the selected marker gets the
      brighter/larger variant (swap BackgroundColor + size); driven from
      `SpaceshipPlayerComponentLock` each frame.
- [x] Focus meter: a thin bar (readout-column style) as a child of the
      reticle indicator in hud/torpedo_target.rs, fill =
      focus.seconds/FOCUS_TIME, visible while a lock exists and focus is
      incomplete, gone once focused (the markers appearing is the completion
      signal). Radial ring stays future polish (needs image/shader tech).
- [x] Tests: markers exist only while focused and match the live section
      count; membership follows section death; highlight follows the
      component lock; meter fraction and visibility windows.
- [x] Extend examples/12_hud_range.rs: after the lock stage, wait past
      FOCUS_TIME and assert the meter filled then vanished, the section
      markers match the target's live section count, and a directly-written
      component lock highlights its marker; after target death assert the
      markers are gone.
- [x] Verify: cargo fmt, cargo check --workspace, new + touched tests, one
      scripted 12_hud_range run under Xvfb (report skips).
- [x] Write the arc doc `docs/2026-07-09-component-lock.md` (or dated when
      it lands): the mechanic end to end, per-consumer behavior deltas
      (enumerated, per the retro lesson), tuning constants with their
      values, and what was deliberately deferred (ring visual, AI component
      picks, faction-based hostility).

## Notes

- Depends on: 20260709-192522 (state to render); the meter also reads
  focus state, so it cannot land before it.
- Marker color must not collide with the amber lead pip or nav cyan; the
  arc doc records the final palette.

## Resolution (20260709)

Shipped: `hud/component_lock.rs` (marker layer via mod.rs observers,
reconcile system keeping one 10 px hot-red Entity-anchored marker per
attached section of the locked ship while focused, highlight system
restyling the selection to 16 px/brighter), the focus meter under the
reticle in `hud/torpedo_target.rs` (48x4 px, fill = focus fraction, visible
only while a lock is held and the dwell is incomplete), 6 new hud tests, and
four new scripted stages in 12_hud_range (meter at 49% mid-dwell with zero
markers; 3/3 markers and no meter after focus; a script-pinned tail section
rendering the highlight; markers gone on target death) - full PASS. Arc doc:
docs/2026-07-09-component-lock.md.

Difficulties: `cargo run` insisted on rebuilding the whole dependency graph
after a fresh `cargo build` of the same target in this worktree, eating the
run timeout twice; sidestepped by executing the built example binary
directly, which then needed `BEVY_ASSET_ROOT=$PWD` because bevy resolves
assets relative to the executable outside cargo. Both recorded for the range
runbook. Also fixed in passing: the ComputedCenterOfMass import in the
example is now cfg(debug)-gated (it was warning in no-feature builds).

Skipped honestly per user instruction: full local suite and clippy (check +
fmt + new/touched tests + the scripted range run).
