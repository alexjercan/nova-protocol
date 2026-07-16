# Make base a first-class dep://base target (root-relative base bundle, Option B)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.7.0,modding,base,feature,spike

## Goal

Make the base game a first-class `dep://base` target - so a mod can reference
base's shipped art with `dep://base/<path>` and base content can use `self://` -
WITHOUT moving any files and WITHOUT breaking the existing bare-path convention.
This is Option B from the spike (tasks/20260716-235458/SPIKE.md): base becomes a
normal scheme-referenceable bundle at low, non-breaking cost.

## Why (from the spike)

`dep://base` is currently rejected because base's `resource_base` is its folder
(`"base"`) while base art lives at the asset ROOT, so it would mis-resolve. base
is also an implicit dependency, never in a mod's `meta.dependencies`. Fixing both
makes base referenceable like any mod. This also fixes a latent bug: base's
current `resource_base="base"` would mis-rewrite any base `self://` ref today.
Bare paths keep working (they coexist with `dep://base`, both resolve to root).

## Direction (for /plan to break into steps)

- Give the base bundle ROOT-relative resolution: its `resource_base` is `""`
  (the asset root), not `"base"`. Decide where - a special-case in
  `BundleAssetLoader`/`register_bundles` keyed on the `base: true` catalog entry
  (or id `"base"`), NOT string-matching everywhere.
- Treat `base` as an ALWAYS-available implicit `dep://` target: allow
  `dep://base/<path>` even though `base` is never in `meta.dependencies`. Remove
  the `id == "base"` REJECTION in `crates/nova_assets/src/mod_refs.rs` and instead
  resolve it against base's root `resource_base` + base's `resources`.
- Add a `resources` list to `assets/base/base.bundle.ron` enumerating base's
  shipped art (the files reachable via `self://`/`dep://base`), emitted by
  `gen_content` so it cannot drift from the builders.
- Mirror across ALL THREE domains, as `self://`/`dep://` already are: runtime
  merge (`register_bundles`), static `lint_walk`, and the engine-free portal
  generator (`nova_portal_gen`). `dep://base` membership is checked against base's
  declared resources; base is SHIPPED so the portal knows its ids but not its
  resources unless the shipped catalog is consulted - resolve how the portal
  validates `dep://base` (or documents the gap, as it does for shipped deps).
- Optionally convert base's OWN content refs to `self://` for provenance (the
  bug fix makes this correct); keep it OPTIONAL - bare still works.
- Docs: guide-make-a-mod, modding-ron, mod-binary-resources design doc - document
  that base is referenceable via `dep://base/<path>` (and bare remains the
  shorthand).
- Tests: `dep://base/X` resolves to root; a declared base resource validates; an
  undeclared one is a gate error in all three domains; bare base refs still work
  unchanged; base `self://` resolves to root (the latent-bug regression).

## Non-goals

- Moving base art into `assets/base/` and dropping the bare convention (Option
  A/C) - deferred per the spike; reconsider after this lands.

## Notes

- Spike: tasks/20260716-235458/SPIKE.md.
- Builds on `self://` (20260716-123544) and `dep://` (20260716-215423).
- Stepless direction-level task: run `/plan` before `/work` to break it into
  ordered steps.

