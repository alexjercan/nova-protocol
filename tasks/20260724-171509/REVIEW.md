# REVIEW: status bar gets its own HudTier::Status (task 20260724-171509)

- BRANCH: feat/status-tier
- ROUND 1: out-of-context reviewer (fresh context; re-derived the visibility
  matrix from source + Bevy semantics; compiled the tests)
- VERDICT: APPROVE (no CRITICAL / MAJOR)

## Verified

1. Visibility matrix: `shown = level.shows(tier) && (!drawer_open || exempt)`
   walked for tier=Status, exempt=true - Inherited at All + Minimal (closed and
   drawer-open), Hidden at None (drawer or not, because `shows` short-circuits
   the whole expression). Matches the cinematic-clear requirement.
2. Objective-hint inheritance: the count is a child of the bar root with no
   `HudTier` and no explicit `Visibility` management, so `apply_hud_visibility`
   never touches it and it inherits the bar's computed visibility. `Display`
   (layout) and `Visibility` (render) are orthogonal, so the hint's Display
   toggle does not fight the inherited visibility - no double-hide.
3. Z-lift: the bar root now carries `HudDrawerExempt` + base `GlobalZIndex`, so
   `lift_exempt_chrome_over_drawer` (a `With<HudDrawerExempt>` presence filter)
   lifts it above the backdrop during the drawer; the drawer panels lack the
   marker so their z is untouched; the child inherits the lifted stacking context.
4. Exhaustiveness/regressions: `HudTier` has no exhaustive `match` anywhere (only
   `shows`'s `matches!` with a wildcard All arm), so the new variant is safe. The
   main menu drives `HudVisibility::None`, at which Status hides - same as the old
   Chrome behavior, no regression.
5. DECISION.md soundness confirmed: reusing the `HudDrawerExempt` presence marker
   for the z-lift (vs a value-filtered `HudTier == Status` query) cleanly excludes
   the drawer panels; the two-orthogonal-axes framing is accurate.

## Findings (both ACTIONED)

- NIT: two stale "Chrome-tier bar root" comments in objective_hint.rs (doc-rot
  from the retag) - updated to "Status-tier".
- MINOR (test gap): the headline composition (the child count inherits the bar
  through the drawer + None) was untested. Added
  `childless_node_is_left_to_inherit_the_status_bar`: pins that
  `apply_hud_visibility` manages only the tiered parent and leaves the child's
  `Visibility::Inherited` untouched, so Bevy propagation carries the bar's state
  to it.
