# Spike: Should turret rounds (bullets) feel gravity wells?

- DATE: 20260712-112113
- STATUS: RECOMMENDED   # conditional - build it, but scoped and guarded
- TAGS: spike, gameplay, gravity, weapons

## Question

Ships and torpedoes opt into `GravityAffected` and curve through wells; turret
rounds and section debris deliberately do not. Task 20260712-105505 asks to
make bullets feel gravity too. This reverses an explicit prior decision, so the
question is threefold:

1. Is the curvature actually perceptible at bullet speeds/ranges near a well,
   or was the original "imperceptible" call correct?
2. What does it cost - per-frame (many live rounds) and in correctness (the
   turret auto-aim assumes straight-line ballistics)?
3. If we do it: bullets only, or debris too? And what has to change besides
   flipping on the marker?

A good answer is a go / no-go / conditional-go with the concrete follow-on
tasks (if go) and the reasons, so `/plan` can expand it without re-litigating.

## Context

- The gravity layer lives in `crates/nova_gameplay/src/gravity.rs`. Any entity
  with `GravityAffected` (and without `GravityWell`) feels the dominant well's
  inverse-square pull each `FixedUpdate` via `gravity_well_system`. The force
  math is mass-independent, so no new physics is needed to include bullets -
  only the opt-in.
- Opt-in today is two observers: `insert_gravity_affected_on_ship` and
  `insert_gravity_affected_on_torpedo`. Adding bullets is a third observer on
  `TurretBulletProjectileMarker` (spawned in
  `crates/nova_gameplay/src/sections/turret_section.rs`).
- The original deferral is decision 5 of
  `tasks/20260709-193147/SPIKE.md`: "Turret
  rounds skip v1: flight times are short, the lead pip assumes straight-line
  ballistics, and per-bullet well queries are pure cost for imperceptible
  curvature." That same spike's open questions already flag "turret lead vs
  curved targets - measure in the turret range before deciding whether the pip
  needs a gravity term," and its deferred list records "turret lead gravity
  term." So this spike is the promised measurement.
- Relevant tunables: bullet `muzzle_speed` 100 u/s, `projectile_lifetime` 5.0s
  (500u max range), `fire_rate` 100 rounds/s. Wells: default surface gravity
  6 u/s^2, capped at 10; `mu = surface_gravity * body_radius^2` (2400-4000 for
  a 20u rock); SOI = 8x body radius (160u for a 20u rock).

## Findings

### 1. Perceptibility (impulse approximation, transverse kick 2*mu/(b*v))

For a bullet at v=100 u/s passing a 20u rock at impact parameter b, asymptotic
deflection angle and the lateral miss it produces at a target 100u past closest
approach:

| surface g | mu   | pass b | dv_perp | angle | miss @100u |
|-----------|------|--------|---------|-------|------------|
| 6 (default) | 2400 | 20u (graze) | 2.40 u/s | 1.38 deg | 2.40 u |
| 6           | 2400 | 50u (orbit band) | 0.96 u/s | 0.55 deg | 0.96 u |
| 6           | 2400 | 100u | 0.48 u/s | 0.28 deg | 0.48 u |
| 10 (cap)    | 4000 | 20u (graze) | 4.00 u/s | 2.29 deg | 4.00 u |
| 10          | 4000 | 50u | 1.60 u/s | 0.92 deg | 1.60 u |
| 10          | 4000 | 100u | 0.80 u/s | 0.46 deg | 0.80 u |

Read: the original "imperceptible" call was **mostly right**. At typical combat
geometry (b >= 50u) the curve is sub-degree and misses by ~1u - below section
scale, barely visible. It only becomes perceptible (2-4u miss, ~1.4-2.3 deg)
when a round grazes the rock surface at the strongest wells. So the honest
verdict is "imperceptible except on close grazing passes," not "always
imperceptible." That thin band of perceptibility is exactly where a curving-
shot mechanic would live - and also exactly where the auto-aim breaks (below).

### 2. Cost

- **Correctness (subtle - may be a net win, not a cost).** The turret intercept
  solver (`lead_intercept_point` / `update_turret_aim_point`) solves a straight-
  line ballistic intercept: `|(target-shooter)+target_vel*t| = speed*t`. It
  models the target as moving at constant velocity and the bullet as flying
  straight. Two things break that in a well:
  - **Pre-existing miss (already shipping).** The target ship IS
    `GravityAffected`, so inside a well it is under acceleration while the
    solver assumes constant velocity - the turret aims BEHIND an accelerating
    target today, before any bullet curves. This is an existing PDC weakness in
    wells, independent of this task. (Reported from playtest, 2026-07-12.)
  - **Bullet gravity may partly CANCEL that miss.** A bullet and a target near
    the same point in the same well feel nearly the same acceleration
    (common-mode); in the relative frame that shared acceleration largely
    cancels, the way you can ignore uniform gravity for short relative
    ballistics. So enabling bullet gravity can pull the effective trajectory
    back toward where the (also-falling) target actually goes - reducing net
    error rather than adding to it. The cancellation is only first-order: it
    degrades when bullet and target sit at very different radii (different
    accel), e.g. a grazing shot at a target far out in the SOI. So the honest
    framing is not "bullet gravity desyncs the aim" (decision 5's worry) but
    "the aim is already desynced by the falling target, and bullet gravity is a
    partial, free correction - measure whether it nets out better." That is the
    headline playtest question.
- **Perf.** `gravity_well_system` is O(wells x affected) with a per-affected
  `Vec` alloc per tick. Affected today is "tens" (ships + torpedoes). One PDC
  at 100 rounds/s x 5s lifetime holds ~500 live rounds; several turrets push
  the affected set into the thousands - a 10-100x jump, plus a `Vec` alloc per
  bullet per tick at 64 Hz. Not catastrophic, but no longer free, and the
  system loops every well for every bullet even for the majority that are in no
  SOI. Needs a cheap reject or it is wasted work on every shot fired anywhere.

### 3. Scope

Debris is a separate axis (decision 5 also defers it, for perf, calling
wreckage-onto-the-rock "a nice later flourish"). It has the same perf shape but
none of the auto-aim correctness problem. Keep it out of this task; leave it as
the already-recorded deferred item.

## Options considered

- **A. Do nothing (keep the deferral).** Cost: none. The measurement above
  largely vindicates the original call, so this is defensible. But the user
  explicitly asked for the feature, and there is real emergent/visual value in
  the grazing band, plus a latent curving-shot mechanic. Rejecting outright
  wastes that.
- **B. Naive on: add the marker, nothing else.** Cheapest to write. But it
  ships the auto-aim desync near wells with no guard and no perf cull - turrets
  quietly get worse next to asteroids and thousands of bullets pay the well
  query every tick. This is the version decision 5 correctly refused.
- **C. Scoped on: marker + perf guard + honest aim posture (recommended).**
  Opt bullets in, but (a) make out-of-SOI rounds cheap so the common case is
  ~free, and (b) take an explicit position on the aim desync rather than
  shipping it silently. Two honest sub-positions on the aim:
  - **C1 (v1):** accept the straight-line lead as intended texture - PDC gets
    slightly less reliable right against a strong rock, which is thematically
    fine and rare - and record a gravity-aware lead solve as a follow-up.
  - **C2:** add a full gravity feedforward to the intercept solve - model BOTH
    the target's acceleration (fixes the pre-existing aim-behind miss) and the
    bullet's, so auto-aim is honest under gravity end to end. Larger scope,
    shared with the STOP/GOTO gravity-feedforward follow-up; better as its own
    task. The user has signalled willingness to invest in this if C1's free
    common-mode cancellation does not net out well enough in playtest.
- **D. Bullets + debris together.** Rejected for this task: debris adds perf
  surface and design questions (wreckage settling on the rock) without the
  gameplay payoff, and muddies the auto-aim discussion. Keep it deferred.

## Recommendation

**Conditional go: Option C, sub-position C1 for v1.** Build it as:

1. Opt turret rounds into `GravityAffected` at spawn (third observer, mirroring
   `insert_gravity_affected_on_torpedo`), and update the now-stale
   `GravityAffected` doc comment that says rounds skip v1.
2. Add a cheap perf guard so bullets outside every SOI cost ~nothing - measure
   first (add a well to `examples/08_turret_range.rs` and check frame cost with
   a full PDC stream), then guard only if the measurement warrants it. Likely
   shape: skip the per-entity `Vec`/well loop when the round is in no SOI, or a
   coarse broadphase against well SOIs. Bullets do not need `DominantWell`
   tracking or hysteresis, so a lighter path than the ship force loop is fine.
3. Accept the straight-line lead pip as-is for v1 and OBSERVE, don't just
   document: the pip already aims behind the falling target, and bullet gravity
   is a free common-mode correction. Playtest whether net PDC accuracy in wells
   improves, stays flat, or worsens. Record the full gravity-aware intercept
   term (C2 - target + bullet acceleration) as a follow-up, paired with the
   already-recorded STOP/GOTO gravity-feedforward follow-up, to build if the
   free cancellation is not enough.

Why C1 before C2: C1 is nearly free and may on its own reduce the existing
aim-behind miss via common-mode cancellation - so it is worth measuring before
committing to the heavier solver work. C2 is a real, shared piece of work
(feedforward into the intercept solve for both bodies) that the user is willing
to fund if the playtest asks for it; sequencing C1 first keeps that decision
data-driven and reversible. Why C over A: cheap to add, gives the emergent
flourish the user asked for, and may fix a shipping PDC weakness for free;
the perf guard turns decision 5's remaining objection into a managed tradeoff.

Explicitly out of scope: debris gravity (stays deferred) and the gravity-aware
lead solve (its own follow-up).

## Open questions

- **Actual per-frame cost of ~500-2000 gravity-affected bullets.** Estimated
  non-trivial but not catastrophic; resolve by measuring in
  `examples/08_turret_range.rs` with a well present before deciding how heavy
  the perf guard must be. This gates step 2's shape.
- **Does bullet gravity net-improve or net-worsen PDC accuracy in wells?** The
  headline playtest question. Baseline: the turret already aims behind an
  accelerating (falling) target because the lead solver assumes constant target
  velocity. Bullet gravity is common-mode with the target's fall and should
  partly cancel that miss - but only to first order, so at grazing geometry
  (bullet and target at very different radii) it may not. Measure net accuracy
  before/after C1. If it does not net out, that is the trigger to build C2 (full
  target + bullet gravity feedforward in the intercept solve), which the user
  is willing to fund.
- **Interaction with bullet lifetime.** At 100 u/s a round crosses a 160u SOI
  in ~1.6s, well under the 5s lifetime, so curvature has time to accumulate on
  a tangential pass; confirm no surprising long-lived orbiting-bullet edge
  cases (a slow or near-tangential round could loiter in-SOI).

## Next steps

Direction-level task this spike seeds, for `/plan` to break into steps:

- tatr 20260712-105505: bullets affected by gravity wells - Option C1 (marker
  opt-in + measured perf guard + honest straight-line-aim posture, bullets
  only). This spike's doc is its Spike reference.

Recorded follow-up, not seeded now (create when C1 lands and a playtest asks
for it):

- Gravity-aware turret intercept solve (C2): add a well feedforward term to
  `lead_intercept_point` for BOTH the target's acceleration (fixes the
  pre-existing aim-behind-a-falling-target miss) and the bullet's, so auto-aim
  stays honest near wells. Pairs with the STOP/GOTO gravity-feedforward
  follow-up from the gravity-wells spike. User has signalled willingness to
  fund this if C1's free common-mode cancellation is not enough in playtest.
