# Auditor's torpedo bay clips inside its hull - mount it on the ship's side

- STATUS: CLOSED
- PRIORITY: 43
- TAGS: v0.7.0,scenario,content,bug

User report (2026-07-17 playtest): "the Auditor's torpedo bay is placed
inside the ship, it should be on its side, it's clipping." The Auditor is
the hostile in webmods/the-ledger/ledger_ch4.content.ron (spawned by two
mutually exclusive ending handlers - fix BOTH spawn sites; the
content_lint dual-spawn warning marks them). Move the tube section to a
side mount so the mesh no longer intersects the hull sections; check the
section grid offsets against how other multi-section ships place side
mounts, and eyeball-verify if a screenshot rig is cheap (render-output-
eyeball lesson) or verify offsets geometrically otherwise. Sibling task
20260717-143806 changes the same ship's gun - coordinate landings.

## Steps

- [x] Generic guard first (lint-covers-types-not-variants: both the
  SpawnScenarioObject and ScatterObjects template paths): a pairwise
  section-overlap check in nova_scenario::lint - sections are unit cubes
  centered on their grid position (base_section's
  Collider::cuboid(1,1,1)), two overlap iff |d| < 1 on ALL axes (flush
  |d| = 1 is legal and common); Error severity. Unit tests: an
  overlapping pair errors, a flush spine passes.
- [x] Fail-first on shipped data: run content_lint BEFORE the content
  fix - the Auditor's tube at (0,0,0.5) (embedded between controller
  and hull_bow) must produce the new ERROR at both spawn sites; record
  the output.
- [x] Fix webmods/the-ledger/ledger_ch4.content.ron at BOTH sites: tube
  -> (1.0, 0.0, 1.0) (flush against hull_bow's +X face) with rotation
  Rz(-90) = (0.0, 0.0, -0.70710677, 0.70710677) so the bay's mount base
  (local -Y) faces the hull it hangs off. Bundle 1.2.0 -> 1.3.0.
- [x] content_lint clean after; sweep every other shipped ship with the
  new lint (it runs repo-wide by construction) and REPORT any further
  hits without fixing them here; note the gunship tubes' base-down
  identity rotation for sibling task 20260717-151214's sweep.
- [x] Docs: CHANGELOG (Fixes: the clipping bay; Internals: the overlap
  lint); NOTES.md with the grid math and the fail-first output.
- [x] Verify: cargo test -p nova_scenario --features serde lint::;
  content_lint; balance gates still green; fmt last. Full suite on CI.

## Close-out record

All six steps landed; grid math, the guard's design and the sibling note
are in NOTES.md. The fail-first output (lint run BEFORE the content fix):
4 ERRORs - 'tube' at (0,0,0.5) overlapping 'controller' (0,0,0) and
'hull_bow' (0,0,1), at both ending-branch spawn sites - and zero hits on
any other shipped ship. Post-fix: content_lint clean, lint unit tests
green (10 in lint::), balance_audit unchanged at 0/0/2 acked, workspace
--all-targets green, fmt last. Full suite on CI per standing instruction.
