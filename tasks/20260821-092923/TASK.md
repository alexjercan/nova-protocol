# A gun round is math, not a physics body

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.11.0, performance, physics

# A gun round is math, not a physics body

Epic: `20260818-220812`. Queued behind `20260821-091044` (env variables).

**The owner's call, 2026-08-21**, taken on the evidence in
`tasks/20260819-173219/notes-fixed-step.md` and D20.

## Why

A 1v1 fight step costs 10.48 ms of a 15.625 ms budget. Avian's actual
pair-and-solve work is only ~30% of that (broad 1.26, narrow 0.96, solver
including all six substeps 1.32). The other 66% is per-step BOOKKEEPING that
scales with body count.

And the bodies are rounds. Measured on the same slice:

- **Contact constraints never exceeded 51, and average two to four.** Almost
  nothing in a fight ever actually touches.
- The 3-4k "contacts" are AABB overlap pairs from speculative margins on fast
  bullets. They cost broad and narrow time and never reach the solver.
- **About 770 rounds are in flight even on a quiet step**, of ~900 dynamic
  bodies.

So six substeps re-integrate ~900 bodies six times over to resolve two contact
manifolds. **The cost is not collision. It is having 900 bodies.**

Estimated win: **2.5-4.5 ms off a 4v4 step, 1.5-2.5 off a 1v1.** That is an
ESTIMATE - arithmetic on measured slices, not a measurement. Treat it as a
hypothesis to test, not a promise. This epic has already had one item ranked
first on an estimate that turned out to be a whole-run total misread as a
per-step one, and it bought nothing.

Why it matters more than its own size: the step cost is a MULTIPLIER on every
other spike, `1 / (1 - step / 15.625)`, which is 2.4x at 9 ms. Shrinking the step
shrinks every hitch in the game.

## Scope: gun rounds only

**Point defence has no separate firing path.** Its module does target assignment
and mount ownership; the rounds come from `shoot_spawn_projectile` in
`crates/nova_ship/src/sections/turret_section/firing.rs`, the same system every
turret uses whether the computer or the player pulls the trigger. So this is ONE
change covering every gun round in the game, and no new "PD round" concept is
needed.

**Torpedoes stay physics bodies.** Owner's reasoning, and it is the right test:
a torpedo is built with the same engine as a ship - thrusters, a PD attitude
controller, sections - so it is a small ship with a bomb on it, not a
projectile. It needs the engine. It is also what point defence shoots at, so
leaving it a body keeps the PD chain untouched.

## It is already almost straight-line

Three things mean this is close to lossless rather than an approximation:

- **`Gravity::ZERO`** (`NovaGameplayPlugin`). There is no arc today to lose.
- **`NEUTRALIZED_BULLET_MASS`** gives a round near-zero mass specifically so
  impact momentum contributes nothing and authored damage is the only source.
- The projectile-hook tests assert the round "flew on UNPERTURBED".

The game already pays to make a bullet behave like a straight line that does not
interact. A swept cast is that, computed directly, with no integration error and
no speculative margin.

## What actually changes, and it is the risk surface

1. **Bullet-versus-bullet pairs disappear.** `ProjectileHooks::filter_pairs`
   rejects exactly one thing - a projectile against the ship that fired it - and
   its comment says "everything else keeps colliding". So ~770 rounds currently
   generate pair work against EACH OTHER. Nobody wants that interaction.
2. **A round stops being pushable.** A blast impulse can nudge one today. After,
   never. Almost certainly invisible; arguably a fix.
3. **Tunnelling becomes impossible** rather than mitigated. The speculative
   margins that prevent it are what produce the phantom contacts.
4. **A round stops being queryable as a collider.** THIS IS THE WHOLE RISK.
   Anything doing a spatial query that expects to find rounds stops finding
   them.

## Enumerate before writing: what depends on a round being a body

First hour of the work, not a separate spike:

- `ProjectileHooks` and `collect_collision_pairs<ProjectileHooks>`.
- `resolve_bullet_hit` (`turret_section/firing.rs`).
- Whatever collision events drive impact effects and hit audio.
- `NEUTRALIZED_BULLET_MASS` - probably becomes moot, but check nothing else
  leans on the term.
- Any spatial query or `Collider` query that can currently return a round.
- The ammo and ownership path: `ProjectileOwner` must still reach
  `HealthApplyDamage.source`, or the AI threat model stops resolving a hit back
  to the shooter.

## Structure it integrate-then-cast

Write the step as: **advance velocity, advance position, cast along the segment
travelled.** NOT a hardcoded `cast from A to A + v*dt`.

This costs nothing today and it is the difference between "add an acceleration
term" and "rewrite it" the day gravity wells arrive. The owner wants curved
rounds eventually - an N-body-ish pull from a small set of wells, not a gravity
simulator - and swept math is the cheap way to get there: analytic acceleration
per round per step, against ~770 rounds, is a handful of vector ops.

**Gravity itself is OUT OF SCOPE here.** Recorded so the seam survives. Note for
whoever picks it up: the hard part of curved rounds is not the projectile, it is
the FIRING SOLUTION - `update_turret_aim_point` solves a straight-line intercept
today, and both the AI gunner and the HUD lead pip read it. Curved rounds with a
straight-line lead means guns that visibly miss.

## Measurement: read the COUNTS before the milliseconds

`stress_point_defense` carries saturation invariants - peak live rounds sit in a
2,419-2,421 band across 16 runs - and that band is what caught the
collision-batching change silently altering outcomes. Swapping a body for a
swept cast is far more likely to move a count than that change was.

- Paired, interleaved arms. Match on body count or fight regime; whole-run
  averages over the arena are not comparable, because a faster simulation ends
  the fight sooner and quietly measures a lighter scene.
- Report the fight-regime per-step median, the 1% low and the worst frame.
- The per-step diagnostics instrument (moving into `nova_probe` under
  `20260821-091044`) is what makes regime selection possible. Use it.
- Never assert a millisecond in a range. Count the thing that causes the cost.

## Definition of done

- No `RigidBody` or `Collider` is spawned for a gun round anywhere.
- `stress_point_defense` still green, with peak live rounds inside its band and
  every outcome marker on the timeline.
- `system_turret_gunnery` green, including its barrel-tracks-the-mover claim.
- Torpedoes and point defence demonstrably unchanged.
- Paired before/after on the fight-regime per-step median, the 1% low and the
  worst frame, on both `stress_point_defense` and a true arena duel
  (`--ship amber --ship onyx`).
- A range that would have gone red on tunnelling before, and does not now, if
  one can be built cheaply.
- `CHANGELOG.md` entry under Performance.
