# Review: Auditor bay mount + section-overlap lint

- TASK: 20260717-151208
- BRANCH: fix/auditor-bay-mount

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) crates/nova_scenario/src/lint.rs:356 - the overlap error
  message literal contains a run of ~26 embedded spaces: `"... overlap
  (unit-cube grid: ..."` is authored as `overlap` followed by 26 spaces and
  then `(unit-cube grid: ...)`. This is inside the string, so cargo fmt does
  not touch it and every emitted lint error carries the gap verbatim (CI
  output, content_lint output). Suggested change: collapse to a single
  space in the format string.
  - Response: fixed - single space in the message literal.

- [ ] R1.2 (NIT) crates/nova_scenario/src/lint.rs:334-352 - the check treats
  sections as axis-aligned unit cubes and ignores `rotation`. That is exact
  for every shipped rotation (all are quarter-turns, under which a cube maps
  to itself), but a section authored with a non-90-degree rotation (the
  config accepts any quat) can physically overlap a flush neighbor (a
  45-degree cube reaches ~0.707 from center in the rotation plane) while
  passing the lint. Errs only toward false negatives, so it is safe as a
  guard; suggested change: one sentence in the doc comment stating the
  axis-aligned / quarter-turn assumption.
  - Response: fixed - the rotation caveat is now in the check's doc.

- [ ] R1.3 (NIT) crates/nova_scenario/src/lint.rs:721-764 - the new unit test
  exercises the overlap check only through the `SpawnScenarioObject` path.
  The scatter-template path is covered by construction (`check_action`
  routes `ScatterObjects` templates through the same
  `check_object_prototypes`, lint.rs:232-236, and the pre-existing
  `unknown_prototype_in_a_scatter_template_is_an_error` test proves that
  plumbing), so this is optional: a one-liner asserting an overlapping
  scatter template errors would pin the claim in NOTES.md directly.
  - Response: acknowledged - the shared-plumbing routing is the pinned
    path (lint-covers-types-not-variants); left as is.

- [ ] R1.4 (NIT) assets/base/sections/base.content.ron:194-198 (pre-existing,
  out of scope for this diff) - the `torpedo_section` prototype's
  `spawn_offset` is `(0, 0, -2)`, i.e. 2 units toward the bow of the BAY,
  not out of the bay's door (the mesh door and the launch axis are both
  local +Y). For the rotated Auditor tube the torpedo therefore materializes
  at ship-local (1, 0, -1) - beside the dorsal turret, forward of the bay -
  and then drifts out along +X. This is strictly better than pre-fix (the
  old tube spawned torpedoes at (0, 0, -1.5), strictly INSIDE the dorsal
  turret's cube) and identical in kind to the shipped gunship, so no change
  requested here; suggested change: add one line to the sibling-task note in
  NOTES.md so 20260717-151214's sweep also considers aligning
  `spawn_offset` with the bay door.
  - Response: acknowledged - noted in NOTES for the sibling turret task's
    sweep; strictly better than pre-fix already.

### Verification record

Re-derived, not taken on trust:

- Avian cuboid convention (THE load-bearing claim): avian3d 0.7.0
  `Collider::cuboid(x_length, y_length, z_length)` takes FULL extents and
  halves them internally - `SharedShape::cuboid(x_length * 0.5, ...)` at
  ~/.cargo/registry/src/index.crates.io-*/avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749.
  `base_section`'s `Collider::cuboid(1.0, 1.0, 1.0)`
  (crates/nova_gameplay/src/sections/base_section.rs:104) is therefore a
  true 1x1x1 cube (half-extent 0.5) centered on the section position. Flush
  spine layouts at distance 1.0 touch and do not penetrate; the lint's
  strict `< 1.0` on all three axes is the exactly correct overlap predicate.
  Float exactness: authored positions are RON literals (0.5, 1.0, ...),
  representable exactly in f32; flush 1.0 - 0.0 == 1.0 is not `< 1.0`.
- Quat convention: file order is (x, y, z, w). Cross-checked against the
  shipped turret_dorsal `(-0.70710677, 0, 0, 0.70710677)` = Rx(-90). The
  tube's `(0, 0, -0.70710677, 0.70710677)` = Rz(-90), which maps local
  +Y -> ship +X and local -Y -> ship -X.
- New position (1, 0, 1) vs every other Auditor section: controller (0,0,0)
  d=(1,0,1) edge contact; hull_bow (0,0,1) d=(1,0,0) FLUSH on its +X face;
  hull_mid (0,0,2) d=(1,0,-1); thruster (0,0,3); turret_dorsal (0,0,-1)
  d=(1,0,2). No pair strictly < 1.0 on all axes; hull_bow flush as claimed.
- Mesh orientation (parsed torpedo-bay-01.glb vertex data, not trusted from
  comments): the bay mesh has a full-footprint flat slab at local
  y in [-1.0, -0.9] (pre-scale) - the MOUNT BASE at local -Y - and the tube
  hatch detail (raised door verts) at y in [0.9, 1.0], local +Y. Node
  transform Ry(180) * scale 0.5 preserves Y. Under Rz(-90) the base (-Y)
  faces ship -X, i.e. seats against hull_bow, and the door (+Y) faces
  ship +X, outboard. The NOTES claim is correct, including the sign: Rz(+90)
  would have pointed the door INTO the hull.
- Launch direction: `shoot_spawn_projectile` launches along the spawner's
  +Y (torpedo_section/mod.rs:625, `projectile_rotation * Vec3::Y`); the base
  prototype's `spawn_rotation` is identity and `spawn_offset` is (0,0,-2)
  (assets/base/sections/base.content.ron:194-204). Composed with the tube's
  Rz(-90): launch = ship +X (outboard, away from the hull at -X); spawn
  point = (1,0,1) + Rz(-90)*(0,0,-2) = ship-local (1,0,-1), outside every
  hull cube (flush against turret_dorsal's +X face, zero penetration), and
  `ProjectileHooks` filters projectile-vs-owner contact pairs anyway
  (crates/nova_gameplay/src/sections/projectile_hooks.rs). Arming
  (arm_time 0.5 / arm_distance 5.0) gates detonation, and the fuze keys on
  the TARGET, not the hull, so no launch-into-own-hull detonation path
  exists. Pre-fix, for contrast, torpedoes spawned at (0,0,-1.5) - inside
  the dorsal turret's cube (R1.4).
- Lint coverage: `check_section_overlaps` is called at the end of
  `check_object_prototypes` (lint.rs:330), which `check_action` invokes for
  both `SpawnScenarioObject` (lint.rs:229-231) and the `ScatterObjects`
  template (lint.rs:232-236). Both paths covered as claimed.
- Test strength: deleting the check fails the overlap half trivially; an
  over-eager `<=` threshold makes the flush case (d=(1,0,0), all axes <= 1)
  error and fails the `issues.is_empty()` half. Both directions pinned.
- Fail-first honesty: `git show
  master:webmods/the-ledger/ledger_ch4.content.ron` contains
  `position: (0.0, 0.0, 0.5)` for id "tube" exactly twice (lines 216, 301 -
  both ending-branch spawn sites). (0,0,0.5) is strictly < 1.0 on all axes
  from BOTH controller (0,0,0) and hull_bow (0,0,1), and from nothing else
  (hull_mid/turret are 1.5 away on z) -> exactly 2 errors per site, 4 total,
  matching the recorded pre-fix output; the unit test proves the code errors
  on the (0,0,0.5) shape. Post-fix content_lint is clean (below).
- Sibling-task note: verified against BOTH the builder source
  (crates/nova_assets/src/scenario/broadside.rs:214-217, the `tube` closure
  uses `rotation: Quat::IDENTITY` at (1,0,-2) and (-1,0,-2), lines 239-240)
  and the generated assets/base/scenarios/broadside_gunship.content.ron
  (tube_port/tube_starboard, identity rotation). The gunship's tubes are
  indeed side-mounted base-down, as NOTES.md records for 20260717-151214.
- CHANGELOG entry matches behavior; bundle 1.2.0 -> 1.3.0 bumped at both
  claims' scope (content-only change).

Commands run (worktree /home/alex/.cache/sprouts/nova-protocol/fix/auditor-bay-mount):

- `cargo test -p nova_scenario --features serde lint::` ->
  `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s`
- `cargo run -p nova_assets --bin content_lint` ->
  `WARN  [the-ledger] scenario 'ledger_ch4_the_buyer': object id 'auditor' is spawned by more than one handler - fine only if the handlers are mutually exclusive`
  `content_lint: clean (1 warning(s))` (the WARN is the pre-existing
  dual-spawn marker called out in the task text)
- `cargo run -p nova_assets --bin balance_audit` ->
  `balance_audit: 11 combat scenario(s), 0 error(s), 0 warning(s), 2 acked`
  (both acks are the 20260717-143806 close-spawn acks; both ch4 sites still
  report `1 tube(s)`, so the moved bay still registers in the threat model)
- `cargo test -p nova_assets --test webmods_validation` ->
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s`
- `cargo fmt --check` -> clean (exit 0)

Per standing instruction (skip-local-tests-and-clippy), the full workspace
suite and clippy were not run locally; only the targeted lint tests plus the
content/balance/webmods gates above. Full suite runs on CI.

No BLOCKER or MAJOR findings; R1.1 is cosmetic-but-real output text, R1.2-R1.4
are optional hardening/notes. APPROVE.
