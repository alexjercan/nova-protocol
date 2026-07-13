# AI threat-scored target selection over the relation model

- STATUS: CLOSED
- PRIORITY: 76
- TAGS: v0.4.0,ai,spike,targeting


Spike: tasks/20260709-225508/SPIKE.md (wave 1)

Goal: replace the hardcoded player Single in input/ai.rs with a per-ship
AITarget(Option<Entity>) picked over hostiles via the relation model
(20260708-203708): score by distance, recent-damage-to-me and target type
(ships over torpedoes over asteroids), with switch hysteresis so the pick
does not flip-flop between frames. All four AI systems (rotation, thrust,
turret target, fire) consume AITarget instead of querying the player.

Blocked on: 20260708-203708 (faction/relation model).
Depends on: 20260709-225726 (state skeleton).

## Steps

- [x] `AITarget(Option<Entity>)` component in `input/ai.rs`, required by
      `AISpaceshipMarker` (default None), registered + in prelude.
- [x] Pure `pick_ai_target(own_anchor, current, candidates)` with tunable
      constants: candidates carry `(Entity, Vec3 position, AITargetKind)`
      (Ship | Torpedo); tiered priority - hostile SHIPS beat hostile
      TORPEDOES regardless of distance (the urgency flip for incoming
      torpedoes is the point-defense task 20260709-225733); nearest wins
      within a tier; max acquisition range constant (2000 m, matching
      TARGETING_MAX_RANGE); switch hysteresis - the current target's
      distance is discounted (e.g. 20%) so a marginally nearer rival does
      not steal the pick frame-to-frame.
- [x] System `update_ai_target` (chained before `update_behavior_state`):
      per AI ship, gather candidates (ship roots + committed torpedo
      projectiles), filter relation(own, candidate) == Hostile via the
      relation model, score with the pure picker, write `AITarget`. Clear
      the target when the entity died or left range.
- [x] Consume `AITarget` everywhere the player Single was: the four AI
      systems look up the target's `(Transform, Option<ComputedCenterOfMass>)`
      and aim at its live-structure anchor; `update_behavior_state`'s
      hostile_present becomes per-ship "has a target". The AI no longer
      references `PlayerSpaceshipMarker` at all.
- [x] Tests: pure picker matrix (tier beats distance, nearest within tier,
      hysteresis holds the current pick, neutral/own never picked, out of
      range -> None); system test - AI ship acquires the hostile ship over
      a nearer hostile torpedo and ignores neutrals; existing AI tests
      updated to drive the AITarget pipeline instead of the player Single.
- [x] Verify: cargo fmt, cargo check --workspace, ai:: module tests (skip
      full local suite per user instruction; report skips honestly).

## Notes

- Relevant files: crates/nova_gameplay/src/input/ai.rs;
  relations.rs (relation model); torpedo markers in
  sections/torpedo_section/mod.rs (TorpedoProjectileMarker,
  TorpedoTargetChosen for the committed gate, mirroring targeting.rs).
- DEFERRED (recorded here): the spike's "recently-damaged-me" threat
  scoring needs damage attribution - `HealthApplyDamage.source` exists in
  bcs but nothing real populates it (only tests trigger damage events in
  nova code; collision/blast damage never sets a source). Wiring
  attribution belongs with the evasion task (20260709-225731), which needs
  the same "I was hit by X" signal; threat scoring joins the picker then.
- Torpedo candidates: only committed ones (TorpedoTargetChosen), matching
  the player targeting rule - a just-launched torpedo is not a target yet.

## Resolution (20260710)

Shipped AITarget (required by the marker, default None), AITargetKind
tiers (Ship > Torpedo, derive(Ord) so the picker is a two-key min), the
pure pick_ai_target (range gate 2000 m, 20% hysteresis discount on the
current pick), the acquisition system over the relation model (hostile
filter, committed-torpedo gate, self-exclusion, live-structure anchors on
both ends), and the consumer swap: all four behavior systems and the
state transition now run off AITarget; PlayerSpaceshipMarker no longer
appears in the AI module outside tests. A shared ai_target_anchor helper
resolves the target's anchor for all consumers. 7 new tests (4 picker,
3 acquisition), existing AI tests updated to drive the real acquisition
pipeline instead of a player Single; full crate suite 208/208 green this
once, fmt + check clean. Skipped per user instruction: clippy.

Deferred, recorded in Notes: recently-damaged-me threat scoring (needs
damage attribution that nothing populates yet; joins the picker with the
evasion task 20260709-225731).

Reflection: the tier-as-Ord-enum trick kept the picker to one min_by with
a lexicographic key - no weights to tune or explain. The consumer swap
was mechanical because 225726 had already funneled every system through
the same (state, anchor) shape.
