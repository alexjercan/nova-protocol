# content_lint: mount-base adjacency check (rotation * -Y must point at an occupied neighbor cell)

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.7.0,scenario,tooling,lint

Seeded by review R1.2 of 20260717-151214: the SECOND wrong-mount-roll bug
in two days (the Auditor bay bottom-down at a side position, then all
four gunship side mounts), and the class is exactly lintable because all
shipped content uses quarter-turn rotations: for every ship section,
rotation * (0,-1,0) is axis-aligned and must point from the section's
cell toward an OCCUPIED neighbor cell (the hull it mounts on). Extend
nova_scenario::lint next to check_section_overlaps (same shared path so
scatter templates are covered). Fail-first: revert one gunship roll or
re-author the old Auditor bay in a fixture. Tolerance for non-quarter
rotations: skip with a note (conservative, like the overlap check's
rotation caveat). Would have caught both shipped bugs at authoring time.
