# Spike: Separate combat vs travel locks; widen the cyclable pool to asteroids

- DATE: 20260712-215256
- STATUS: SUPERSEDED (2026-07-13, by tasks/20260713-082207/SPIKE.md)
- TAGS: spike, targeting, navigation, hud

> SUPERSEDED: the cyclable-pool direction (A1, task 20260712-215402) was
> closed wontdo by the user - scroll-based target cycling is out entirely.
> The combat/travel separation question this spike opened was answered
> through 20260712-215733 and 20260712-222610, and finally by the
> deliberate-radar model (20260713-082207).

## Question

Now that CTRL+scroll cycles targets (task 20260712-212742), three coupled asks:

1. Re-add asteroids and other larger signed bodies to the CYCLABLE pool at
   longer ranges, so you can flick to a far body you cannot aim at precisely -
   but NOT unsigned debris (annoying at 500+ m).
2. At really long ranges, "LOCK" a CLUMP of asteroids as a single travel
   waypoint.
3. The root question: the one lock resource does double duty - a sticky COMBAT
   target and an aim-driven TRAVEL (GOTO) designator - and these want different
   feels. How would we separate a combat mode from a travel mode?

User steer: keep it combat-mode-only FOR NOW; this spike should still sketch the
combat/travel split in a sentence or two and recommend a near-term direction for
the asteroid re-add. A good answer is a direction + seeded tasks, not code.

## Context (input/targeting.rs, input/player.rs)

- One lock resource, `SpaceshipPlayerTargetLock`, is consumed by BOTH the
  auto-turret aim feed (combat) AND `AutopilotAction::Goto` (travel/GOTO,
  player.rs:848) and torpedo designation.
- The candidate collection range-gates everything by the LockSignature scanner
  model: ships / gravity wells at full range (`TARGETING_MAX_RANGE` 20 km);
  signed bodies (asteroids carry `LockSignature(radius)`, beacons authored) at
  `signature * signature_range_per_unit` (30/u, so a r=10 rock locks at 300 u, a
  r=50 at 1500 u); committed torpedoes at `torpedo_lock_range` (2500);
  UNSIGNED debris only point-blank (`unsigned_lock_range` 15). So "big things at
  range, debris only up close" already falls out of the signature gate - the
  user's "not debris at 500 m" line is the signed/unsigned line.
- The AIM pick (cone) runs over ALL collected candidates when no combat lock is
  held, so you already DESIGNATE a beacon/asteroid for GOTO by aiming at it.
- The CTRL+scroll cycle set / candidate HUD / edge indicators
  (`rank_combat_targets` -> `entries`) is filtered `is_hostile &&
  is_combat_target` (ships + committed torpedoes). Asteroids/beacons are NOT in
  it - they are neutral and non-combat.
- The sticky `held` gate holds only combat targets, precisely so nav bodies stay
  aim-re-designatable (task 20260712-203353 review R1.1).
- Clutter is real: asteroid fields (04_asteroids and scenarios) can have many
  signed rocks; the cycle is capped at 5 (`TARGET_CANDIDATE_COUNT`) and the edge
  overlay points at every member.

## Options considered

### Part A - widen the cyclable pool to asteroids

- **A1. Add signed NON-debris bodies to `entries` as NON-sticky, combat-first
  cycle members (recommended near-term, small).** Drop the `is_hostile` gate for
  entries in favour of `is_hostile && is_combat_target` OR (signed &&
  signature >= a threshold); keep `held` (sticky) COMBAT-ONLY. So CTRL+scroll
  walks ships, torpedoes, and notable asteroids/beacons; a cycled asteroid rides
  the existing 4 s `pinned_until` window (long enough to press G for GOTO) but is
  not sticky, so aiming still re-designates. Debris is excluded for free (no
  signature). Clutter guarded by a signature/size threshold + reserving cap
  slots for combat targets.
  - Pros: reuses the pin; reaches far bodies you cannot pixel-aim; the
    signed/unsigned line matches the user's "no debris". Cons: mixes nav into the
    combat cycle/HUD (a field can still crowd the 5-cap and edge overlay -
    needs the threshold + combat-first cap, a tuning risk); a half-measure the
    Part C split supersedes.
- **A2. Do nothing (aim-only for nav).** You can already aim at a big asteroid to
  GOTO it; the cycle stays clean combat. Cheapest. Con: a far body that is
  sub-pixel on screen is hard to aim at - the same legibility gap the target
  inset addressed - so "cycle to it" has real value.
- **A3. A separate TRAVEL cycle (this is Part C).** Cleanest but new mechanism.

### Part B - asteroid-clump travel locks

- **B1. Synthesize a "clump" target from spatially-clustered asteroids
  (centroid + aggregate signature), lockable/GOTO-able at long range.** Pros:
  "lock the field to fly there" without picking one rock. Cons: a clustering
  pass, a synthetic entity's lifecycle (forms/dissolves as rocks move/die), and
  GOTO to a moving centroid - real cost. Future, not near-term.
- **B2. Author explicit region/waypoint markers in scenarios** (a big invisible
  signed nav body per cluster). Simpler, static, no clustering - but hand-placed,
  not emergent. A cheap stand-in if B1 proves heavy.
- **B3. Do nothing** - GOTO an individual rock in the field. Loses the "travel to
  the field" intent at range.

### Part C - combat vs travel separation (sketch, per user steer keep for later)

- **C1. Mode toggle** (a key flips Combat <-> Travel): one lock, the mode
  swaps the eligible pool (combat: ships/torpedoes, sticky; travel: wells,
  beacons, asteroids, clumps, aim/cycle) and the stickiness. Least new state.
- **C2. Two lock resources** (`CombatLock` + `TravelLock`) with distinct HUD and
  consumers (turrets read combat, GOTO reads travel). Cleanest separation, most
  HUD/consumer plumbing.
- **C3. Context-auto** (no explicit mode): nav bodies enter the cycle only when
  no combat target is near / only past some range. No new input, least
  predictable.

## Recommendation

Keep it combat-mode-only now, as steered. Two-line split sketch: the cleanest
future separation is a **Combat/Travel mode toggle (C1)** - one lock, the mode
decides the eligible pool and whether it is sticky (combat sticky, travel
aim/cycle) - because it needs the least new state and no second reticle, and it
maps to how the player already thinks ("I am fighting" vs "I am going
somewhere"). Two locks (C2) is more correct but more HUD/consumer plumbing than
this earns yet; revisit if the toggle feels modal.

Near-term, buildable: **A1** as a small, reversible widening - notable signed
bodies (asteroids/beacons above a signature threshold) join the CTRL+scroll
cycle as non-sticky, combat-first entries, so you can flick to a far body for
GOTO; debris stays out by construction. Treat the field-clutter guard
(threshold + combat-first cap) as the main risk and a playtest knob. This is a
stopgap the Part C toggle later subsumes, and it is worth landing on its own.

Clumps (B1) are a real but heavier idea: seed it, do NOT block the near-term
work on it, and consider B2 (authored regions) as the cheap first cut if B1's
clustering proves costly.

## Open questions

- The clutter threshold: which signature/size counts as "notable" enough to
  cycle, and does the 5-cap reserve slots for combat vs nav? Decide at plan time
  against 04_asteroids and a real field scenario.
- Does A1's non-sticky-but-pinned asteroid feel right for GOTO (4 s to press G),
  or does travel want its own longer/again-sticky hold? A playtest question,
  and an input into whether C1 (mode) is needed sooner.
- B1 clump lifecycle: form/dissolve hysteresis, GOTO to a moving centroid, and
  whether clumps are combat-relevant at all (probably travel-only).

## Next steps

One near-term task seeded (v0.5.0); the other two directions live in this doc
as future work, not separate tasks yet (user steer: keep it light for now):

- tatr 20260712-215402: cyclable nav bodies - notable signed asteroids/beacons
  join the CTRL+scroll cycle at range (non-sticky, combat-first, no debris).
  THE ONE seeded task.
- FUTURE (not seeded - this doc is the record): combat vs travel lock
  separation (mode toggle, C1) - the root fix; keep combat-only until it earns
  a task. And asteroid-clump travel waypoints (Part B) - long-range GOTO to a
  cluster; consider authored regions (B2) as the cheap first cut. Seed these
  when they are next up.

## Addendum (2026-07-12, spike 20260712-215733)

A newer user steer supersedes parts of this doc; the unified-target-computer
spike (tasks/20260712-215733/SPIKE.md) is the current
source of direction:

- A1's NON-STICKY detail is superseded: one lock for both classes, sticky for
  both, cone membership as the only gate. Task 20260712-215402 was repurposed
  in place to carry that unified-list change (its body records the shift).
- The Part C separation got a fuller UX analysis there ("Combat vs travel
  separation" section): the leading future shape is a VIEW-ROUTED two-slot
  lock (travel lock in Normal/FreeLook, combat lock while RMB/Turret view;
  guns read only the combat slot) rather than the C1 toggle key - it also
  solves "traveling to a friend must not point the guns at them". C1's
  toggle-key form is effectively retired as the recommended shape.
- Part B (clumps) is untouched and would plug into the travel slot.

Second addendum, later the same day: the view-routed split arrived
immediately rather than as future work - spike
tasks/20260712-222610/SPIKE.md (travel lock in
Normal/FreeLook + combat lock raised via RMB, seed-on-raise,
fire-gated-on-lock) is now the current source of direction, seeding tasks
20260712-223034/223035/223036/223345. Task 20260712-215402 was closed
unstarted, its scope redistributed there. Part B still plugs into the
travel slot when it comes up.

## Fix record

(none yet - tasks not started)
