# Auditor bay mount + section-overlap lint - design record

Task 20260717-151208. User report: the Auditor's torpedo bay clips
inside the ship; it should be on its side.

## The bug and the fix

Sections are unit cubes centered on their authored grid position
(base_section's Collider::cuboid(1,1,1)). The Auditor's tube sat at
(0, 0, 0.5) - strictly inside BOTH the controller (0,0,0) and hull_bow
(0,0,1) cubes. It now mounts at (1.0, 0.0, 1.0), flush on hull_bow's +X
face, rotated Rz(-90) ((0, 0, -0.70710677, 0.70710677)) so the bay's
mount base (local -Y) seats against the hull instead of hanging
bottom-down while side-mounted. Both mutually exclusive spawn sites
fixed; bundle 1.2.0 -> 1.3.0.

## The generic guard (and its fail-first)

check_section_overlaps in nova_scenario::lint: pairwise, two sections
overlap iff their centers are strictly closer than 1.0 on EVERY axis;
flush contact (exactly 1.0 on some axis) is the normal layout and
passes. It runs through the shared check_object_prototypes path, so both
direct spawns AND scatter templates are covered
(lint-covers-types-not-variants). Fail-first on SHIPPED data, recorded:
run before the content fix, content_lint produced exactly 4 errors (the
tube vs controller and vs hull_bow, at both spawn sites) and NOTHING
else repo-wide - every other shipped ship is grid-clean, which is also
this cycle's sweep result.

## Noted for the sibling turret task (20260717-151214)

The broadside gunship's two torpedo tubes at (+-1, 0, -2) carry IDENTITY
rotation - side-mounted with their bases pointing down, the same
base-not-toward-hull pattern the user reported for its turrets. In its
sweep, consider rolling the tubes with the turrets.

## Verification

- Fail-first: content_lint 4 errors pre-fix (output in TASK close-out),
  clean post-fix (pre-existing dual-spawn WARN only).
- lint unit tests: overlapping pair errors, flush mount passes.
- balance_audit still 0 errors / 0 warnings / 2 acked (the bay move does
  not change armament or distances materially: 301.5u -> the tube rides
  the same ship).
- cargo test -p nova_scenario --features serde lint:: green; workspace
  --all-targets green; fmt last. Full suite on CI.
