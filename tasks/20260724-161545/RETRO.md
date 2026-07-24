# RETRO: objective hint becomes a status-bar block (task 20260724-161545)

- OUTCOME: CLOSED, review APPROVE round 1, probe playable OK.
- BRANCH: feat/objective-hint-status-item (one squash commit).

## What changed and why

The top-right objective hint floated as its own absolutely-positioned node and
overlapped the fps/version status bar (both hug the top-right corner). Moved the
hint INTO the bcs status-bar row so it flows beside fps + version and can never
overlap them. Owner chose a plain-text look (dropped the bordered TAB pill and
the gold glyph square) to match the other bar items.

## Decisions

- The bcs status registry (`insert_status_bar_item`) only builds the fixed
  icon+prefix+value+suffix TEXT schema and hands back a SEPARATE unmarked visual
  child - so a literal `status_bar_item` would strip the two markers the hint
  needs (the reveal tuck anchor source + hide-at-0). Chose to parent our OWN
  plain-text node as a child of `StatusBarRootMarker` instead: same visual result
  (a plain-text block flush in the bar row), nova-only (no bcs change), and it
  keeps `ObjectiveHintMarker`/`ObjectiveHintCountMarker`. This is a deliberate
  reading of "make it a registry block" - in the bar, laid out by the bar, but
  not via the text-only auto-insert.
- Hide-at-0 switched from `Visibility::Hidden` to `Display::None`. As a flex
  sibling a hidden-but-laid-out node leaves a GAP in the row; `Display::None`
  removes it from layout so the bar closes up.
- Dropped `HudTier::Chrome` + `HudSelfDrivenVisibility` from the hint. It now
  inherits visibility from the Chrome-tier bar root, so the grave/tilde HUD cycle
  (and, once 134335 lands, the drawer hide) still hides it - one axis instead of
  two.

## Difficulties / notes

- The overlap was NOT caused by this sprint's drawer work - it was a pre-existing
  collision between the objective hint (task 134312, floating top:16 right:8) and
  the fps/version bar (bcs, top:10 right:10). Diagnosed by mapping every
  top-of-screen widget's anchor rather than assuming the reported "status item"
  was something newly added.
- New coupling: the hint now hard-depends on the status bar existing. Acceptable
  (nova_core spawns the bar for every app), handled by a graceful bail + warn if
  absent, and the test rig now spawns a bar root.

## Self-reflection / for next time

- When a playtest reports "the thing YOU added overlaps X", verify attribution
  before accepting it - here the true cause was a sibling task's widget, and the
  fix belonged to neither the drawer task nor a bcs change. Mapping the anchors of
  every widget in the affected screen region is the cheap diagnostic.
- The bcs status registry's config-entity -> separate-visual-child split is a
  sharp edge: you cannot put your own markers on a `status_bar_item`'s rendered
  node. If a future widget needs BOTH the registry's dynamic-value plumbing AND
  its own markers, that split forces either a bespoke child (this task) or a bcs
  extension. Worth a ledger note.

## Ledger candidate

- `bcs-status-item-is-config-not-visual`: `status_bar_item` / the
  `StatusBarItemMarker` auto-insert consumes the spawned entity as a config SPEC
  and builds a SEPARATE unmarked visual child under the bar root - so you cannot
  attach your own markers/behaviour to a registry item's rendered node. For a
  status-bar block that needs its own markers (e.g. an anchor source, a custom
  hide rule), parent a bespoke node under `StatusBarRootMarker` directly.
  20260724-161545.
