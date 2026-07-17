# AI line-of-sight fire gate: hold fire and reposition when cover occludes the target

- STATUS: CLOSED
- PRIORITY: 54
- TAGS: spike,v0.7.0,ai,gameplay,balance

Goal: make cover a real pressure-relief mechanic. AI turrets hold fire while
a tangible (non-Sensor) blocker occludes the aim point, and the ship uses its
existing approach/orbit machinery to regain the angle. This is NOT an AI
nerf: aim, lead prediction and damage are untouched; the AI stops wasting
ammo into rocks and visibly maneuvers for the shot. Today the fire decision
never raycasts (crates/nova_gameplay/src/input/ai.rs, fire gate ~:1391-1466),
so hiding behind an asteroid stops bullets (they expend on it,
turret_section.rs:431) but never stops the pressure.

Direction notes:
- Gate FIRING only, not target acquisition; dropping targets on occlusion
  would read as dumber AI and is out of scope.
- Recommend exempting point-defense (anti-torpedo) fire from the gate.
- Raycast via avian SpatialQuery from muzzle toward the LEADED aim point;
  mind perf (per firing turret, not per frame per turret if avoidable) and
  the two-clocks lesson (FixedUpdate vs render poses, docs/LESSONS.md).
- The mechanic is symmetric (enemies behind cover also stop eating player
  fire only if authored so) - decide and test the player-side effect.

Spike: tasks/20260717-111808/SPIKE.md (findings F3/F4; Options B)

## Steps

- [x] LOS gate in `on_projectile_input` (crates/nova_gameplay/src/input/ai.rs:1391):
  after the range and alignment gates pass and only when NOT point-defending
  (PD bypass), `SpatialQuery::cast_ray_predicate` from the muzzle toward the
  LEADED aim point, max_distance = muzzle-to-aim distance; the predicate
  treats Sensor colliders and the shooter's own colliders
  (`ColliderOf.body == shooter root`) as transparent; a hit whose
  `ColliderOf.body != gun_target` holds fire (explicit `**input = false`).
  Verified API: avian3d-0.7.0 spatial_query/system_param.rs:176
  (predicate false = collider skipped, ray continues),
  collider_hierarchy/mod.rs:53 (`ColliderOf { body }`),
  `SpatialQuery` needs `Res<ColliderTrees>`. System runs in `Update`
  (ai.rs:41-60), trees are one physics tick stale - acceptable for a fire
  gate, note it in a comment.
- [x] Same-helper LOS gate for AI torpedo launches
  (`update_torpedo_target_input`), bay muzzle toward target, keeping the
  existing AI_TORPEDO_* envelope; if the code shape makes this
  disproportionate, skip and record why here.
- [x] Sweep every existing rig that runs `on_projectile_input`
  (ai.rs tests ~:2037,:3278,:3294,:3303,:3344,:3505 - re-grep, do not trust
  this list) for the new `SpatialQuery` param; init the missing avian
  resources (lesson: required-component-in-shared-query).
- [x] Integration tests on a production-faithful rig (PhysicsPlugins app,
  real ship/collider hierarchy, real colliders; lesson:
  production-faithful-rigs): (a) tangible blocker between AI muzzle and
  target -> turret input false, then remove the blocker and step -> input
  true in the SAME test (delivery guard); (b) Sensor volume between -> still
  fires (the beacon-area lesson from despawn_bullet R1.1 applies to rays);
  (c) ray meets the target's own collider -> fires; (d) PD defending with a
  blocker -> still fires. Sabotage: with the gate condition disabled, test
  (a) must go red (would-it-fail-without-it); record the failing output.
- [x] Docs in the same task (keeping-docs-in-sync): CHANGELOG.md entry
  (player-visible behavior change), the dev wiki page that describes AI
  behavior (check web/src/wiki/dev/ map first), tasks/<id>/NOTES.md design
  record (decision, alternatives, the leaded-ray subtlety, PD bypass).
- [x] Verify: cargo fmt + cargo check --all-targets, run the NEW tests
  (`--features serde` unification trap does not apply to nova_gameplay, but
  grep docs/LESSONS.md for the crate name before crate-scoped runs); full
  suite stays on CI per user instruction - say so in the report.

## Close-out record

All six steps landed as planned; the torpedo gate shipped (not skipped).
What changed: `ai_line_of_fire_blocked` helper + gates in
`on_projectile_input` (PD-exempt, runs after all cheaper gates) and
`update_torpedo_section_input` (one ray per ship); 5 new tests in
`line_of_fire_tests` on the shared physics harness; 4 bare rigs gained
`ColliderTrees`; CHANGELOG + player wiki (combat-weapons.md, new "Cover &
line of fire" section) updated; design record in NOTES.md.

Verification: cargo fmt; cargo check --workspace --all-targets green;
`cargo test -p nova_gameplay input::ai::` 88 passed / 0 failed (includes
the 5 new tests and every pre-existing rig of the two changed systems).
Sabotage A/B recorded in NOTES.md (2 red under a neutered gate, restored).
Full test suite intentionally left to CI per standing user instruction.

Reflection: two traps cost a compile round each - `ColliderTrees` is not in
avian's prelude, and `TorpedoSectionConfigHelper` cannot be constructed in
tests (private field; use the production `torpedo_section()` bundle). Both
now recorded in NOTES.md. Reading avian's cast_ray_predicate SOURCE (not
its doc comment) up front avoided a real semantic bug: predicate-false
means skip-and-continue, not stop.
