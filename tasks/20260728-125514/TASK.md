# NOVA OS ship app: minimize + make readable the blip overlay (tiny label, ammo as numbers, defer detail to side panel)

- STATUS: CLOSED
- PRIORITY: 28
- TAGS: v0.9.0,feedback,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

As a player using the `ship` app, I want each blip's on-view overlay to be
minimal and readable, so the schematic is not cluttered with tiny green-on-green
text - the FULL detail belongs in the side inspector panel, not floating on the
3D view.

Playtest verdict (2026-07-28) on the landed legibility change (`20260728-115435`):

- KEEP: "the cubes look really good, I like a lot the idea of having a smaller
  cube inside a frame that WORKS" - the uniform-green fill + bright wireframe
  outline + gap is a win; do not touch it.
- FIX 1 (readability): "the text makes it hard to read, it's really small and
  green on green" - the blip label + integrity bar text is hard to read.
- FIX 2 (ammo): "the ammo count is weird because it's literally 500 circles :D" -
  a high-capacity weapon renders a pip per round. Use NUMBERS instead (or rethink)
  so a large magazine reads cleanly.
- FIX 3 (minimalism): "the first two should be extremely minimal because we will
  have the side panel anyway" - the blip overlay should carry the bare minimum; a
  label with the section name + some way to tell the section KIND makes sense, but
  the detail (integrity %, ammo, status words) moves to the side panel.

## What it should do

- Cut the per-blip overlay down to the essentials: a readable label (section
  name/code + a kind indicator) and keep it legible against the phosphor scene
  (contrast, a backing/outline, or show label only for the SELECTED/hovered blip
  rather than all at once - decide at plan time).
- Replace the ammo PIP row with a compact numeric readout (e.g. `2/6`) or drop
  ammo from the blip entirely and leave it to the side panel.
- Reduce or remove the per-blip integrity bar if the side panel already carries
  status - the blip should not duplicate what the panel shows. Decide how much
  the blip keeps once the panel exists.
- Do NOT change the block rendering (fill + wireframe outline + gap) - that is the
  part the owner explicitly likes.

## Approach

Confirmed with the owner (see `DECISION.md`, which supersedes the blip-status part
of `20260728-115435`): strip the per-blip integrity bar + ammo pips (the inspector
panel from `20260728-115430` now holds HP/ammo/status detail, killing the "500
circles"). Each blip becomes a minimal marker: a status-coloured DOT (green
nominal -> amber critical, so damage is glanceable across the ship) plus a label
(kind glyph + code) on EVERY blip, made readable with a dark backing pill + brighter
text. Selection stays the amber border. Blocks/outlines are untouched (owner likes
them).

## Steps

- [x] In `spawn_ship_blip` (`crates/nova_gameplay/src/hud/nova_os_ship.rs`): remove
      the integrity-bar (track + fill) and ammo-pip children. Keep the dot; set its
      background to `status_color()` and the amber border for selection. Wrap the
      label in a dark backing pill (a `Node` with `BackgroundColor` ~
      `NOVA_OS_SCREEN` alpha, small padding + `border_radius`) with brighter
      (`NOVA_OS_TEXT`) glyph+code text for contrast.
- [x] Shrink `ShipBlip` back to `{ section: Entity }` (drop `bar_fill` / `ammo`).
- [x] In `project_ship_blips`: drop the bar-width / bar-colour / ammo-text updates;
      update the dot's `BackgroundColor` to `status_color()` each frame (so damage
      recolours it) and keep the position / visibility / selection-border logic.
      Remove the query params that only served the bar/ammo.
- [x] Remove the now-dead `ShipSectionView::bar_fraction` and `ammo_pips` helpers
      (nothing uses them once the bar/pips are gone; the panel shows ammo as
      `rounds/capacity`).
- [x] Update tests: delete `integrity_bar_and_ammo_pips_track_live_data` (its
      helpers are removed) and retarget `blip_carries_kind_glyph_and_integrity_bar`
      -> `blip_is_status_dot_with_labelled_marker` (assert the dot's
      `BackgroundColor` == `status_color()` for a critical section, the label reads
      `"<glyph> CODE"`, and there is NO bar/ammo child).
- [x] Verify with a NON-test `cargo check` (dead_code lint active - the removed
      helpers/fields must leave nothing dangling), the ship tests, and a GPU
      screenshot.

## Definition of Done

- The blip no longer renders an integrity bar or ammo pips; there is no per-round
  pip rendering left (cmd: `grep -n "ammo_pips\|bar_fill\|bar_fraction" crates/nova_gameplay/src/hud/nova_os_ship.rs`
  returns nothing; manual: no "500 circles").
- Every blip shows a readable label (kind glyph + code) with a dark contrast
  backing; the panel remains the home for HP/ammo/status detail
  (test: `blip_is_status_dot_with_labelled_marker`; manual: labels readable, not
  tiny green-on-green).
- The dot colour reflects section status (nominal green -> critical amber) so a
  damaged section is spottable without selecting it
  (test: `blip_is_status_dot_with_labelled_marker` asserts dot bg == `status_color`;
  manual: a critical section's dot reads amber).
- No dead code from the removed helpers/fields; the palette stays phosphor/amber
  (cmd: `cargo check -p nova_gameplay 2>&1 | grep -c "never read"` prints `0`;
  cmd: `grep -c "srgb" crates/nova_gameplay/src/hud/nova_os_ship.rs` unchanged at 1).

## Work Log (close-out)

**What changed** (`crates/nova_gameplay/src/hud/nova_os_ship.rs`, per `DECISION.md`
which supersedes the blip-status part of `20260728-115435`):

- `spawn_ship_blip`: removed the integrity-bar (track + fill) and ammo-pip
  children. The dot's `BackgroundColor` is now the section's `status_color()`
  (recoloured each frame), with the amber border for selection. The label (kind
  glyph + code) sits in a dark backing pill (`NOVA_OS_SCREEN` alpha 0.82 + padding
  + radius) with brighter `NOVA_OS_TEXT`, fixing the tiny green-on-green.
- `ShipBlip` shrank to `{ section }` (dropped `bar_fill` / `ammo`).
- `project_ship_blips`: dropped the bar-width / bar-colour / ammo-text updates and
  the `q_text` param; now recolours the dot's `BackgroundColor` by status each
  frame and keeps position / visibility / selection-border.
- Removed the now-dead `ShipSectionView::bar_fraction` and `ammo_pips` helpers.
- `SHIP_BAR_PX` const removed. Blocks/outlines untouched.

**Tests.** Deleted `integrity_bar_and_ammo_pips_track_live_data` (its helpers are
gone); retargeted the blip test to `blip_is_status_dot_with_labelled_marker`
(asserts the dot bg == `status_color()` amber for a critical turret, the label
reads `"<glyph> CODE"`, and NO `●`/`○` pips remain). 15 `nova_os_ship` tests pass.

**Dead-code gate.** Applied last cycle's `dead-code-hides-under-cfg-test-reader`:
ran a NON-test `cargo check -p nova_gameplay` (exit 0, 0 `never read`) so the
removed helpers/fields left nothing dangling. `cargo fmt` clean.

**Visual verification.** `screenshot_nova_os` (real GPU, exit 0): the blips are now
status-coloured dots with `@ CTL-1`-style labels on dark backing pills (readable),
no bars, no pips; the inspector panel is unchanged. Directly resolves the "500
circles" + green-on-green playtest feedback.

## Notes

- The overlay lives in `spawn_ship_blip` / `project_ship_blips` in
  `crates/nova_gameplay/src/hud/nova_os_ship.rs`: today each blip is a dot + a
  glyph label + an integrity bar (track+fill) + (weapons) an `ammo_pips()` line.
  `ammo_pips()` renders one `●`/`○` per round - that is the "500 circles".
- Depends on / coordinates with the side-inspector-panel task
  (`20260728-115430`): the panel is where the full detail goes, which is WHY the
  blip can shrink. Sequence this after (or alongside) the panel so we do not strip
  the blip before the detail has a home.
- Load-bearing forks to settle at plan time (surface to owner): (a) show labels
  for ALL blips or only the selected/hovered one; (b) does the blip keep any
  status cue (small bar / dot colour) or defer status entirely to the panel; (c)
  ammo as blip number vs panel-only. These change WHAT is built, not just styling.
- Depends on: 20260728-115430. Follows `20260728-115435`.
