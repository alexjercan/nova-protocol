# REVIEW: objective hint becomes a status-bar block (task 20260724-161545)

- BRANCH: feat/objective-hint-status-item
- ROUND 1: out-of-context reviewer (fresh context; re-derived every claim from
  source + Bevy 0.19 semantics; ran the objective_hint tests)
- VERDICT: APPROVE (no CRITICAL / MAJOR / MINOR)

## Verified

1. Overlap fixed: the hint is spawned via `commands.entity(bar).with_children(..)`
   where `bar` is the `StatusBarRootMarker` - the SAME root fps/version parent to
   via bcs `insert_status_bar_item`. So the hint is a real flex sibling in the
   top-right Row and structurally cannot overlap the version. Node metrics match
   the bcs item (height 24, margin 4, row, center), so it sits flush.
2. Parenting/timing: `setup_status_ui` runs `OnEnter(GameAssetsStates::Loaded)`,
   before any scenario spawns the player ship, so the bar root exists when
   `setup_hint` fires. `q_bar.single()` bail is panic-free on 0 or >1 roots
   (strictly safer than bcs's own `Single` which panics on >1).
3. Hide-at-0 via `Display::None`: correct to avoid a gap (a `Visibility::Hidden`
   flex child still occupies layout). Dropping `HudTier`/`HudSelfDrivenVisibility`
   is correct - the hint inherits visibility from the Chrome bar root, so it
   still hides at `HudVisibility::Minimal`/`None` (and, once 134335 lands, on
   drawer-open). The two axes (Display for count, inherited Visibility for the
   tier) compose without conflict.
4. Tuck anchor: a reveal only spawns at count > 0 (Display::Flex), so the hint is
   laid out and its `GlobalTransform` is meaningful when the anchor matters; the
   reveal re-reads the anchor every frame, so first-frame layout lag self-corrects.
5. Lifecycle: `remove_hint` despawns the marked entity on ship Remove; Bevy 0.19
   `despawn()` is recursive and maintains the parent's `Children` - no orphan.
6. Tests non-vacuous: `hint_parent_is_bar` asserts the hint's `ChildOf` equals the
   bar root; the collapse test proves Display None -> Flex -> None across
   objective changes. `ChildOf` tuple access correct for this version.

## Nits

- (ACTIONED) stale struct doc on `ObjectiveHintMarker` ("top-right row (glyph...)")
  - updated to "The hint block in the status bar (count + TAB, plain text)".
- (left) test rig spawns a bare `StatusBarRootMarker`, not real fps/version items;
  sibling-ship is proven structurally (same parent the items use). Optional
  strengthening only.
- (left) `HINT_ANCHOR_SIZE` is a fixed nominal rect; the anchor uses the
  translation + this size, not the laid-out width. Pre-existing, unchanged.
