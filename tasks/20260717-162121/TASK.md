# content_lint: mount-base adjacency check (rotation * -Y must point at an occupied neighbor cell)

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.7.0, scenario, tooling, lint

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

Verified at plan time (branch feat/lint-mount-adjacency):

- The mount base IS local -Y for Turret and Torpedo section kinds, from
  the GLB vertex data (20260717-151208 review, re-read in
  tasks/20260717-151214/NOTES.md - hatch and turntable at +Y); the
  torpedo launch kick is local +Y (torpedo_section/mod.rs:625), the
  turret yaw axis local Y (turret_section.rs:582).
- "Every ship section" in the seed prose is too wide: shipped THRUSTERS
  are identity-rotated at the spine's +Z end (-Y points at empty space),
  and hull/controller sections are symmetric. The check applies to
  Turret/Torpedo kinds only, per the seeding review R1.2's own wording.
- The occupied neighbor can be ANY section, not just hulls: most shipped
  ships seat the aft turret base against the CONTROLLER cell (turret at
  (0,0,-1), base +Z, controller at origin). Full 28-ship survey of
  assets/base + assets/mods + webmods: every shipped mount satisfies
  `position + rotation * -Y` lands on a sibling section's cell, so HEAD
  lints clean.
- lint_scenario only gets prototype IDS (HashSet<String>); the mount
  check needs section KINDS. Both production callers (runtime
  register_bundles at nova_assets lib.rs:753, static lint_walk) hold
  full SectionConfigs before collapsing to ids.
- ScenarioObjectConfig embeds in exactly two config paths - 
  SpawnScenarioObject (actions.rs:41) and ScatterObjects.template
  (actions.rs:2354) - both already flow through check_object_prototypes
  (lint-covers-types-not-variants swept).

Steps:

- [x] KnownSections in nova_scenario::lint (named SectionCatalog until
  review R1.2 hit the nova_assets::balance name collision): { ids,
  mounts } sets + from_configs(&SectionConfig iter) classifying
  Turret/Torpedo as mounts (conservative on cross-bundle id conflicts:
  all defs must be mount-kind); lint_scenario takes &KnownSections in
  place of known_sections.
- [x] check_mount_adjacency beside check_section_overlaps, called from
  check_object_prototypes: for each mount section (Prototype in
  catalog.mounts, or Inline with mount kind), base_dir = rotation *
  NEG_Y; if not axis-aligned (|component| snap eps 1e-4) -> Warn note,
  skip; else Error unless another section sits within 1e-3 of
  position + base_dir.
- [x] Callers: register_bundles builds KnownSections::from_configs
  (outcome.sections); lint_walk's WalkedBundle keeps Vec<SectionConfig>
  and lint_bundle feeds base + own + deps configs into from_configs;
  lint_walk inline tests updated.
- [x] Tests (lint.rs): clean side mount (Rz roll), clean top/bow mounts,
  gunship-bug shape (identity side turret) errors, Auditor shape
  (bottom-down side tube, non-overlapping) errors, non-quarter rotation
  warns + skips, Inline mount checked, catalog classification unit
  test. Existing fixtures swept: all use empty mount sets, no new
  firing.
- [x] Fail-first A/B on real content: revert ONE gunship roll in
  crates/nova_assets/src/scenario/broadside.rs, gen_content,
  content_lint must ERROR; restore, regen byte-identical, clean run.
- [x] Docs same task: CHANGELOG line; wiki pages listing content_lint
  checks (modding-ron.md / guide-author-scenario.md / scenario-system.md
  - check keeping-docs-in-sync.md map) gain the mount-base rule and the
  authoring convention (mount base = local -Y).
- [x] Verify: fmt last, cargo check --workspace --all-targets, cargo
  test -p nova_scenario --features serde lint, content_lint_gate +
  content_ron_parity, content_lint + balance_audit bins clean.
