# Objective hint becomes a plain status-bar item (fix top-right overlap with the version)

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

# Objective hint becomes a plain status-bar item (fix top-right overlap with the version)

## Goal

Playtest (owner, 2026-07-24): the top-right objective HINT (task 20260724-134312:
glyph + count + TAB pill) overlaps the fps/VERSION status bar - both float in the
top-right corner (hint at top:16 right:8; the bcs status bar at top:10 right:10),
so the hint draws over the version.

The hint should be a BLOCK IN THE STATUS BAR (the bcs status registry), not a
separate absolutely-positioned node, so it flows in the bar's flex row and can
never overlap the version. Owner choice: PLAIN TEXT look (drop the bordered TAB
pill and the gold glyph square), matching the other bar items (fps, version).

## Design (owner gate 2026-07-24)

- LOOK: plain text - count + "TAB" as plain text, gold count color, NO pill, NO
  glyph square. Matches the fps/version items.
- PLACEMENT: a block in the status bar so it lays out in the flex row (no overlap).
- IMPLEMENTATION: parent the hint's node as a CHILD of `StatusBarRootMarker`
  rather than the literal `status_bar_item`/`StatusBarItemMarker` auto-insert.
  Reason: the bcs registry consumes the spawned entity as a config spec and
  builds a separate visual child carrying none of our markers, but the hint needs
  its OWN node markers for (a) the reveal's tuck anchor (`DrawerTabAnchor` from
  the hint rect, task 20260721-211520) and (b) hide-at-0. Parenting our own
  plain-text node keeps both, stays nova-only (no bcs change), and reads as a
  plain text block in the bar.
- HIDE-AT-0: the hint collapses when there are no objectives. As a flex child,
  Visibility::Hidden leaves a gap, so toggle `Node.display` None/Flex (not just
  Visibility) so the bar closes up cleanly.

## Scope / notes

- Files: hud/objective_hint.rs (the widget), nova_core/src/lib.rs setup_status_ui
  (where fps/version items + the bar root live), bcs status.rs (registry, read-only).
- The tuck anchor (`DrawerTabAnchor`) must still centre on the hint's new location
  so the diegetic reveal keeps tucking correctly.
- Interacts with 20260724-134335 (drawer-open HUD hide): the bar + hint are Chrome
  and hide together on drawer open - keep that.

## Definition of Done

1. The objective hint renders as a plain-text block inside the status bar row,
   right of / beside fps + version, never overlapping the version (manual: owner
   opens a scenario with objectives).
2. Hide-at-0 collapses the block with no gap in the bar (test: display toggles).
3. The reveal's tuck anchor still centres on the hint's new rect (test:
   `update_tab_anchor` from the child node; the diegetic reveal still tucks).
4. `cargo check -p nova_gameplay` + `cargo fmt --check` clean; probe playable OK.
