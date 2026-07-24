# DECISION: objective hint as a bespoke child of the status bar, not a status_bar_item

- DATE: 20260724
- STATUS: ACCEPTED (owner gate 2026-07-24)
- TASK: 20260724-161545

> Backfilled after the fact (2026-07-24): this load-bearing build-shape choice
> shipped with its rationale only in the TASK body + RETRO. Recording it here as
> the canonical DECISION.md per the flow/plan skill's mandatory-record rule.

## Context

The top-right objective hint (glyph + count + bordered TAB pill) floated as its
own absolutely-positioned node and overlapped the fps/version status bar (both
hug the top-right corner). The fix is to move the hint INTO the bcs status-bar
row so it flows beside fps/version and can never overlap. The owner chose a
plain-text look (drop the bordered TAB pill and the gold glyph square) to match
the other bar items.

The bcs status registry (`insert_status_bar_item`, fired by `StatusBarItemMarker`)
only builds a fixed icon+prefix+value+suffix TEXT schema, and it consumes the
spawned entity as a config SPEC while building a SEPARATE unmarked visual child
under the bar root. So a literal `status_bar_item` cannot carry the two markers
the hint needs on its rendered node.

## Options weighed

- **Literal `status_bar_item` (registry auto-insert)** - most "registry-native",
  but strips the hint's own markers: it could not publish the reveal's tuck
  anchor (`DrawerTabAnchor`, from the hint's rect) nor drive hide-at-0, because
  those need `ObjectiveHintMarker`/`ObjectiveHintCountMarker` on the rendered
  node. Rejected.
- **Extend bcs to hold custom-content blocks** - a registry that accepts
  pre-built children; keeps the pill and is fully registry-native, but a bcs
  change + pin bump the owner did not want here. Deferred.
- **Bespoke child of `StatusBarRootMarker` (CHOSEN)** - parent the hint's own
  plain-text node as a direct child of the bar root. Same visual result (a plain
  text block flush in the flex row), nova-only (no bcs change), and it keeps the
  markers for the anchor + hide-at-0.

## Decision

Parent the hint as our own plain-text child of `StatusBarRootMarker` (count +
`TAB`, no pill/glyph). Visibility is INHERITED from the bar root (the hint carries
no `HudTier`). Hide-at-0 toggles `Node.display` None/Flex (not `Visibility`), so a
flex sibling does not leave a gap when there are no objectives.

## Consequences

- The hint is a real flex sibling of fps/version, so it structurally cannot
  overlap the version.
- It is "in the status bar" but NOT a formal `StatusBarItemMarker` - a deliberate
  reading of "make it a registry block" given the registry's text-only schema.
- The hint hard-depends on the status bar existing; handled by a graceful bail +
  warn if the bar root is absent.
- Superseded-adjacent: task 20260724-171509 later retagged the bar root
  `Chrome -> Status`; the hint (a child) inherits that, so it now persists through
  the drawer and clears only at cinematic `None`. That is a change to the PARENT's
  tier, not to this decision.
