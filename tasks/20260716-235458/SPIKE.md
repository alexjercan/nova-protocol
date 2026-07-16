# Spike: make base a normal mod (art under `assets/base/`, `self://` + `dep://base`)

- DATE: 20260716-235458
- STATUS: RECOMMENDED
- TAGS: spike, modding, base, refactor

## Question

Should the base game be treated as "just a mod", Factorio-style - its art living
under `assets/base/` and referenced with `self://` (base's own content) and
`dep://base/<path>` (dependent mods) - instead of the current model where base
art lives at the asset ROOT and everything references it with bare, scheme-less
paths? And if so, how, given the pipeline that just shipped
(`self://` task 20260716-123544, `dep://` task 20260716-215423)?

A good answer names the coherent end-states, weighs the migration/ergonomic cost
against the (aesthetic) benefit, and lands on a recommended direction concrete
enough to plan.

## Context (grounded in the code)

- **Base art is at the asset ROOT.** `assets/gltf/` (5 meshes), `assets/textures/`
  (cubemap, cubemap_alt, asteroid + `.meta` sidecars), plus `icons/`, `sounds/`,
  `shaders/`. `assets/base/` holds ONLY content RON (`base.bundle.ron`,
  `scenarios/`, `sections/`) - no binary art.
- **Bare == base game.** A scheme-less path resolves against bevy's default source
  (`assets/` root). Base content uses ~37 bare refs (`crates/nova_assets/src/lib.rs`
  `scenario_generation` constants; `crates/nova_assets/src/sections.rs`
  `SectionMeshRefs::from_paths`, e.g. `"gltf/hull-01.glb#Scene0"`).
- **Two loading surfaces**, not one:
  1. Content `AssetRef`s -> rewritten at merge (`register_bundles`) then
     `AssetRef::resolve` -> `asset_server.load(path)`
     (`crates/nova_gameplay/src/asset_ref.rs:95`). `self://`/`dep://` live here.
  2. The `GameAssets` bevy_asset_loader collection - ~9 HARDCODED bare paths in
     Rust (`crates/nova_assets/src/lib.rs:989-1021`, `#[asset(path="textures/cubemap.png")]`)
     loaded at startup, NEVER through the merge. A file move must update these too.
- **`gen_content`** writes base content from the `.rs` builders; the bare paths
  are constants there, so emitting `self://` + a `resources` list is a bounded
  edit (constants + ~3 builder fns + a generated resources list).
- **Every shipped/web mod already reuses base art via bare paths**: `example`
  (`gltf/hull-01.glb`, `textures/cubemap.png`), `gauntlet` and all four
  `the-ledger` chapters (`textures/cubemap.png`, `textures/asteroid.png`,
  `textures/cubemap_alt.png`) - ~7 mod files, dozens of refs. This is the decisive
  fact: **bare-is-base is a pervasive, relied-on ergonomic**, not a wart.
- **`dep://base` is currently REJECTED** (`crates/nova_assets/src/mod_refs.rs`,
  the `id == "base"` arms) because base's `resource_base` is its folder (`"base"`)
  while base art is at root, so `dep://base/textures/x` would wrongly resolve to
  `base/textures/x`. `base` is also an IMPLICIT dependency - never listed in a
  mod's `meta.dependencies` - so the "declared dependency" gate would reject it
  anyway.

## Options considered

### A. Full physical relocation + namespacing (the literal ask, max Factorio)

Move the ~37 art files under `assets/base/`; base content emits `self://`; base
declares a `resources` list; update the `GameAssets` Rust paths to `base/...`;
`gen_content` emits `self://`; reverse the `dep://base` rejection; migrate all 7
mods' bare base-art refs to `dep://base/...` (and either drop bare entirely for
true canonical uniformity, or keep it).

- Pros: maximal purity - base is a self-contained bundle folder, the asset root
  is clean (`base/`, `mods/`, catalog), and every ref is namespaced (Factorio's
  `__base__/` model). One canonical way if bare is dropped.
- Cons: **breaks every shipped mod** (bare base refs stop resolving) unless bare
  is preserved; taxes the most common case (reusing a stock hull/skybox) with a
  `dep://base/` prefix + declaring `base` as a dependency; base must enumerate its
  whole art set in `resources` and the membership gate now polices base's own art;
  touches content + Rust `GameAssets` + `gen_content` + `meta_check` config + docs
  + 7 mods. Large, breaking, mostly-aesthetic.

### B. Root-relative base bundle - `dep://base` works WITHOUT moving files (recommended core)

Leave base art at the root. Make the base bundle resolve mod-relative refs
against the ROOT: give base `resource_base = ""` (empty) and treat `base` as an
always-available IMPLICIT `dep://` target. Then `dep://base/X` and base's own
`self://X` both rewrite to `X` (root) - exactly where base art already is. Add a
base `resources` list (the art files it "ships" at root) so the membership gate
has something to check. Bare paths are untouched and keep working.

- What it delivers: base becomes a first-class, scheme-referenceable bundle - the
  CONCEPTUAL "base is a mod" purity - with zero file moves and zero broken mods.
  Base content MAY use `self://` for explicit provenance; mods MAY use
  `dep://base/X` for explicit provenance + membership validation (a typo'd
  `dep://base/textures/astaroid.png` is caught at lint/portal/runtime, whereas a
  bare typo just fails to load). Also fixes a LATENT bug: base's current
  `resource_base="base"` would mis-rewrite any base `self://` ref today.
- Cost: bounded and non-breaking - remove the `dep://base` rejection, wire base
  as an implicit dep with `resource_base=""` + its `resources`, add the base
  `resources` list (via `gen_content`), gate `dep://base` on base being implicit
  (not requiring `meta.dependencies` to list it). Mirror in the three domains as
  usual.
- Cons: base art still sits at the root (cosmetic), and there are now TWO ways to
  reference base art (bare AND `dep://base`) - additive uniformity, not canonical.

### C. Move art under `base/` + a fallback `AssetReader` (non-breaking relocation)

Move base art under `assets/base/` (physical purity) but register a custom bevy
`AssetReader`/source so a bare `textures/x.png` transparently falls back to
`base/textures/x.png` - so the `GameAssets` collection and all 7 mods keep working
unchanged.

- Pros: achieves the physical relocation non-breakingly.
- Cons: a fallback reader is real complexity and risk - it interacts with `.meta`
  sidecar loading, the web IndexedDB reader, and `meta_check` paths, and makes
  asset resolution non-obvious (a bare path could come from root OR `base/`). It
  trades one "special" (bare == root) for another (a magic fallback). Arguably
  LESS clean than the explicit convention it replaces.

### D. Do nothing

Keep the shipped model: bare == base at root, `self://` == own folder, `dep://`
== non-base packs; base stays special.

- Pros: zero churn/risk; the common case (reuse base art) stays free; the model
  is already coherent and shipped.
- Cons: base is not a "pure" self-contained bundle; base art clutters the root;
  the base/mod asymmetry persists; `dep://base` stays unavailable.

## Recommendation

**Do Option B; defer A and C.**

The purity idea is worth pursuing, but its *value* is conceptual uniformity and a
little extra validation - it unlocks no new capability (mods already reach base
art for free via bare paths, and `dep://` already handles non-base shared packs).
Option B captures essentially all of that value - base becomes a first-class
`self://`/`dep://base` bundle, with membership validation and a latent bug fixed -
for a **bounded, non-breaking** change that moves no files and breaks no mod. It
beats A (which breaks all 7 mods and taxes the common case for a cosmetic root
cleanup) and C (which buys the same cosmetic cleanup with a risky fallback
reader).

The one thing B does NOT deliver is the literal "art files live inside
`assets/base/`" and a single canonical reference style (bare dropped). Those are
**aesthetic/organizational** goals with real cost (A/C) and no functional payoff,
so they should be a separate, deliberate decision AFTER B is in hand and the team
can feel whether the root clutter or the two-ways-to-reference actually bothers
anyone. B is also a clean stepping stone: once `dep://base` works, a later
relocation only has to move files + repoint, not invent the scheme.

## Open questions

- **Canonical vs additive.** B leaves bare AND `dep://base` both valid. Is that
  acceptable, or does "purity" require ONE way (drop bare -> full A with a mass
  mod migration)? This is a preference call for the user; B is reversible either
  way.
- **How much of base's art to declare.** B needs a base `resources` list. List
  only art referenced via a scheme, or the full shipped set? Full set is safest
  for the gate but larger; `gen_content` can emit it either way.
- **Physical relocation later (A/C).** Still wanted for a clean root once B ships?
  If yes, C's fallback-reader feasibility (meta sidecars, web reader) needs its
  own spike before committing.

## Next steps

Direction-level task this spike seeded (for `/plan` to break into steps):

- tatr 20260717-000416: make `base` a first-class `dep://base` target (Option B) -
  root-relative base bundle, implicit-dep gate, base `resources` list, three-domain
  mirror, docs.

Deferred (NOT seeded): the physical relocation of base art into `assets/base/`
(Option A/C) - reconsider after B ships; needs the canonical-vs-additive decision
and, for C, a fallback-reader feasibility spike.

## Fix record

(Single seeded task; this section stays empty unless the relocation follow-up is
later seeded and this becomes a multi-task family.)
