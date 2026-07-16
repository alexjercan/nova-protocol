# Spike: make base a normal mod (art under assets/base/, self:// + dep://base)

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.7.0,modding,spike,base,refactor

## Resolution

Spiked - see tasks/20260716-235458/SPIKE.md (STATUS: RECOMMENDED). Recommendation:
do Option B (make base a first-class `dep://base` target via a root-relative base
bundle - no file move, non-breaking), defer the physical relocation of base art
into `assets/base/` (Options A/C) as a mostly-cosmetic, breaking follow-up.
Seeded task 20260717-000416.

## Idea (from the user, 2026-07-16)

Lean into the "base is just a mod" principle all the way, Factorio-style: the
base game's own art (gltf, textures, skyboxes, audio) lives UNDER `assets/base/`
and is referenced with `self://` from base's own content, and mods reach shared
base art with `dep://base/<path>`. Base becomes a self-contained bundle folder
that ships everything, not a special root with bare-path magic.

## Why this is a spike, not a plan

It reopens a decision task 20260716-215423 deliberately closed: `dep://base` is
currently REJECTED and bare (scheme-less) paths mean "base game asset at the
asset root". Flipping to the purity model is a large, breaking change with real
tradeoffs; decide IF and HOW before committing. Land on a direction (do it /
don't / partial), then seed the implementation tasks.

## Current model (what we'd be changing)

- Base art lives at the asset ROOT: `assets/textures/`, `assets/gltf/`, etc.
- Base content and mods reference it with BARE paths (`"gltf/hull-01.glb#Scene0"`,
  `"textures/cubemap.png"`). Bare == base game. This is the single load-bearing
  convention across ALL existing content (base + every mod).
- Base's `resource_base` is its folder (`base`), but base art is NOT under
  `assets/base/`, so `self://`/`dep://base` would MIS-resolve today - which is
  exactly why `dep://base` is rejected and base uses bare paths.
- See docs/design/mod-binary-resources.md ("Cross-mod references") and
  crates/nova_assets/src/mod_refs.rs (RefScope, the base rejection).

## Questions to answer

- Does base art actually move under `assets/base/`, or do we keep it at root and
  only change the referencing convention? (The former is the "pure" version.)
- If base art moves: EVERY existing bare ref breaks. Options:
  1. Rewrite all base + mod content to `self://` (base) / `dep://base` (mods).
  2. Keep bare paths working as a compatibility alias for `base/` (a resolver
     shim: a scheme-less path falls back to `base/<path>` if not at root). Then
     the migration is non-breaking and `dep://base` becomes additive.
- Reverse the `dep://base` rejection (it only becomes coherent once base art is
  under `base/`). What are the new gates - is `base` a declared dependency
  (implicit) that `dep://base` is allowed to name? Does base declare a
  `resources` list (all its art), and does the membership gate scale to base's
  whole asset set?
- `gen_content` writes base content from the `.rs` builders - it would have to
  emit `self://` paths and a base `resources` list. How much churn?
- Ceremony cost on the COMMON case: today reusing a stock hull mesh is a free
  bare ref; under the pure model a mod must declare `base` as a dependency and
  write `dep://base/gltf/hull-01.glb`. Is that acceptable, or do we keep bare as
  the zero-ceremony path for base and reserve `dep://` for non-base packs?
- Namespacing/tidiness win (base art under `base/`, no root clutter) vs. the
  migration + ceremony cost. Is the purity worth it?

## Prior analysis (starting point, not a conclusion)

Task 20260716-215423's session argued AGAINST migrating base art: bare paths
already give every mod free, zero-ceremony access to base art, and the pure
model breaks every bare ref and adds declaration ceremony to the most common
case, for a mostly-aesthetic gain. The compatibility-alias option (bare ==
`base/` fallback) was NOT explored and could make it non-breaking - that is the
most promising thread to chase in this spike.

## Output

A SPIKE.md with a recommendation (do / don't / partial + the alias approach) and,
if "do", the seeded implementation tasks (migration, `dep://base` support,
`gen_content` changes, docs, mod-content sweep).

## Notes

- Builds on the landed `self://` (20260716-123544) and `dep://` (20260716-215423)
  pipelines.
