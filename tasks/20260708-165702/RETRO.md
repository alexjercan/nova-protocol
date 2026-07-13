# Retro: Locked-target info readout (HUD)

- TASK: 20260708-165702
- BRANCH: weapons-hud (shared arc branch)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 2 NIT, both addressed; round 2
  APPROVE)

The smoothest cycle of the arc; the substrate absorbed everything hard. What
shipped is in the task's Resolution.

## What went well

- **Plan-time attachment decision held.** Child-of-reticle at `left: 100%`
  was decided when planning (spike had left it open) and delivered exactly
  the promised freebies: edge tracking under ApparentSize scaling and
  visibility inheritance, leaving the readout with zero projection or
  visibility code.
- **One enum component instead of two marker types.** The
  `TorpedoTargetReadoutLine` enum turned "update two text lines" into a
  single query with no disjointness gymnastics - two `Query<&mut Text,
  With<A>>`/`With<B>` params would not have compiled without `Without`
  filters. Worth reusing whenever several sibling nodes differ only by role.
- **The example asserted semantics, not just presence.** Parsing the shown
  distance back and comparing against the actual separation, and asserting
  the closing-speed sign flip under the real approach burn ("CLS  -0.0" at
  rest -> "CLS +13.5 u/s" burning), proved the math conventions live, not
  just that text appeared.

## What went wrong

- **A guarded-write inconsistency shipped to review (R1.1).** The Text
  writes were guarded on inequality but the health-fill Node/color writes
  were not - the exact change-detection lesson from the substrate's R1.1,
  applied to one component and forgotten on the neighbor three lines down.
  Root cause: applying a review lesson as a point fix rather than as a rule
  for the whole system being written.
- **A borrow-order slip cost one compile cycle.** `world.entity(
  player_root(world))` in the example: the helper takes `&mut World` inside
  an immutable borrow. Trivial, but the second time this session that
  example code compiled only on the second attempt; scripted-example
  helpers that take `&mut World` compose badly inline - bind first.

## What to improve next time

- When a review lesson is "guard writes for change detection", sweep the
  whole system for unguarded writes before handing back, not just the line
  the finding pointed at.
- In `&mut World` script code, bind helper results to locals before starting
  entity borrows.

## Action items

- None new. The arc's follow-ups already exist: bevy_common_systems
  promotion (20260709-164608), turret target-velocity feed
  (20260709-173700).
