# Base as a first-class implicit dep://base target (Option A, mechanism)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.7.0,modding,base,feature

## Goal

Make `base` an ALWAYS-available implicit `dep://base` target so a mod can write
`dep://base/<path>` (and base content can use `self://`) - the mechanism half of
Option A (tasks/20260716-235458/SPIKE.md), before any files move. base is NOT
listed in a mod's `meta.dependencies` (it is the implicit universal dependency);
`dep://base` must be allowed anyway.

## Context

Task 20260716-215423 REJECTS `dep://base` (the `id == "base"` arms in
crates/nova_assets/src/mod_refs.rs) because base art is at the asset root while
base's `resource_base` is its folder (`"base"`). Option A moves base art UNDER
`assets/base/` (task 2), which makes `resource_base = "base"` correct - so
`dep://base/textures/x` -> `base/textures/x` -> `assets/base/textures/x`. This
task lands the gate/rewrite mechanism (proven with SYNTHETIC bundles, like the
existing dep:// gate tests); task 2 makes it resolve against real moved files.

## Steps

- [ ] Remove the `id == "base"` REJECTION in `mod_refs.rs` (`rewrite_leaf` and
      `violation`); treat `base` as declared+available when it is present in the
      scope's deps map.
- [ ] Gate logic: `dep://base/X` is allowed even though `base` is never in
      `declared_deps` - base is the implicit universal dep. Other ids still
      require a `meta.dependencies` entry.
- [ ] `register_bundles`: add `base` (the `base: true` catalog entry / id
      "base") to EVERY owning bundle's deps map (its `resource_base` +
      `resources`), so `dep://base` resolves and is membership-checked.
- [ ] Mirror in the static `lint_walk` and the engine-free `nova_portal_gen`
      (base implicit-allowed; membership against base's declared resources; the
      portal knows base only from the shipped catalog - resolve how it validates
      `dep://base`, or document the gap as it does for shipped deps).
- [ ] Unit + synthetic-bundle tests: `dep://base/X` resolves against base's
      folder; a declared base resource validates; an undeclared one is a gate
      error in all three domains; `dep://base` needs no `meta.dependencies` entry.
- [ ] Decision record: update SPIKE.md (mark Option A chosen over B; FIX the
      "declare base as a dependency" error - base stays implicit).

## Notes

- Spike: tasks/20260716-235458/SPIKE.md (Option A).
- Base's `resource_base` stays `"base"`; it becomes correct once task 2 moves the
  art under `assets/base/`. Do NOT make it root-relative (that was Option B).
- Builds on self:// (20260716-123544) and dep:// (20260716-215423).
