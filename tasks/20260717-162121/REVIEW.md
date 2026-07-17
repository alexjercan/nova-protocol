# Review: content_lint mount-base adjacency check

- TASK: 20260717-162121
- BRANCH: feat/lint-mount-adjacency

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; the MINOR and NITs below are left to
  the implementer, who has opted to address them - Round 2 verifies)

Review basis: shared-session risk mitigated with an out-of-context review
pass (fresh-context agent over the raw diff + spec) whose load-bearing
claims were then re-verified in-session. Independent verifications
performed:

- Geometry re-derived BY HAND for 7 shipped mounts across 3 files
  (broadside_gunship's four side mounts, shakedown's and ledger_ch4's aft
  turrets, the Auditor's flank tube) from the raw RON quaternion tuples:
  every `position + rotation * -Y` lands on a real sibling cell
  (controller or hull), none pass by accident. Epsilons check out:
  authored `0.70710677` quats give base-direction components within ~1e-7
  of integers (AXIS_EPS 1e-4), integer-grid positions make cell matches
  exact (CELL_EPS 1e-3).
- Mutation analysis of the new tests: (a) deleting check_mount_adjacency,
  (b) inverting the occupancy predicate, (c) erroring the non-quarter
  branch, (d) classifying thrusters as mounts - each mutant is killed by a
  named assertion; no vacuous test.
- Caller semantics: runtime gate's known-id set is identical to the old
  HashSet collect (merged overlay is last-wins by id, so contested ids
  cannot occur at runtime); static walk's visible set is the same
  base + own + declared-deps union as before; audit_bundles' sections are
  the same Content::Section items in the same order - balance_audit
  inputs unchanged (11/0/0/2 acked, verified by run).
- Independent fail-first A/B (beyond the implementer's builder-side one):
  editing the GENERATED RON's turret_starboard roll to identity turned
  content_lint_gate red with exactly the mount-base error, restored clean.
- Docs: all four quaternion literals in guide-author-scenario.md's new
  sharp-edges bullet verified by computation; CHANGELOG's "build time and
  in-game mod gate alike" claim matches the two callers in the diff.
- Test runs observed: nova_scenario lint 21/21, nova_assets --lib 82/82,
  content_lint_gate 2/2, parity 2/2, workspace --all-targets clean (only
  the pre-existing proc-macro-error2 future-incompat note).

- [x] R1.1 (MINOR) crates/nova_scenario/src/lint.rs:616 - a NON-UNIT
  authored quaternion defeats the quarter-turn predicate: `q * v` is only
  a rotation for unit q, and RON quats are hand-typed by mod authors. A
  Rz(-90) quat scaled by sqrt(2) gives `base_dir = (-2, 1, 0)`, which
  rounds to itself and PASSES the deviation test with a non-unit snapped
  vector - the check then targets a cell 2 away (a spurious Error, or a
  lucky wrong pass). Note: the out-of-context pass also claimed a zero
  quat self-satisfies via target == own position; that sub-claim is WRONG
  (glam's mul_vec3 with q = 0 returns v itself, i.e. behaves as identity),
  recorded here so the corrected derivation is the one on file. Suggested
  change: after snapping, additionally require snapped to be a unit axis
  vector (exactly one component of magnitude 1, rest 0) and route
  failures into the existing Warn branch, wording it "non-quarter-turn
  (or non-unit) rotation".
  - Response: fixed (60508ee9) - the Warn branch now also fires when
    `snapped.length_squared() != 1.0` (exact: components are rounded
    integers), message reworded as suggested; the warn test gained the
    `Quat::from_xyzw(0, 0, -1, 1)` (sqrt(2)-scaled Rz(-90)) case, which
    without the guard proceeds to occupancy with target (-1, 1, 0) and
    errors - so the mutant is killed by the errors-empty assert.
- [x] R1.2 (NIT) crates/nova_scenario/src/lint.rs:35 - `SectionCatalog`
  collides with the pre-existing, unrelated
  `nova_assets::balance::SectionCatalog` (a HashMap<String,
  SectionConfig>); no import clash today, but two public types with one
  name across adjacent crates invites confusion (register_bundles already
  fully qualifies). Suggested change: rename the lint type (e.g.
  `KnownSections`).
  - Response: fixed (60508ee9) - renamed to `KnownSections` with fields
    `ids` + `mounts` (mirrors the `known_scenarios` parameter it sits
    beside); repo-wide grep shows the only remaining `SectionCatalog` is
    the pre-existing balance.rs one plus this review's own text and the
    TASK.md plan history (amended to note the rename).
- [x] R1.3 (NIT) crates/nova_scenario/src/lint.rs:38-41 - the
  conservative contested-id rule is documented, but not the asymmetry it
  creates: a mod overriding a base hull id with a turret kind is
  under-flagged by the STATIC walk (union of visible defs) yet classified
  accurately by the RUNTIME gate (merged overlay), so such content can
  pass CI and still be refused in-game. One sentence in the doc comment
  naming the runtime gate as the accurate classifier closes the surprise.
  - Response: fixed (60508ee9) - the `mounts` field doc now states the
    static walk is the under-flagging side and the runtime merge gate
    classifies from the actual last-wins overlay.
- [x] R1.4 (NIT) crates/nova_scenario/src/lint.rs (tests) - the "ANY
  sibling counts, not just hulls" occupancy rule has no direct unit test
  (shipped content only exercises controller/hull neighbors; the clean
  fixture uses a hull). Cheap add: one case seating a mount base-against
  another mount.
  - Response: fixed (60508ee9) - `mount_seated_against_another_mount_is_clean`:
    hull at origin, inner mount seats against it, outer mount seats
    against the INNER MOUNT; asserts zero issues. A kind-filtered
    occupancy (hulls only) would error on the outer mount, so the test
    pins the kind-blind rule.

## Round 2

- VERDICT: APPROVE

All four Round 1 findings verified against commit 60508ee9:

- R1.1: the Warn branch's guard now requires a unit-axis snapped vector
  (`snapped.length_squared() != 1.0`, exact on rounded integers); the
  sqrt(2)-scaled Rz(-90) case is in the warn test and the mutant-kill
  derivation (guard removed -> occupancy targets (-1, 1, 0) -> Error ->
  errors-empty assert fails) holds.
- R1.2: rename complete; repo grep shows no `SectionCatalog` outside
  balance.rs and the task-record prose.
- R1.3: asymmetry sentence present on the `mounts` field doc.
- R1.4: `mount_seated_against_another_mount_is_clean` present and green;
  it fails under a kind-filtered occupancy.

Checks after the fixes: nova_scenario lib lint 22/22, nova_assets --lib
82/82, content_lint_gate 2/2, `cargo check -p nova_scenario -p
nova_assets` clean, fmt run last. No new findings.
