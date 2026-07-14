# Spike: bundle family v2 - content-declared kind + folder bundles + generic registration

- DATE: 20260714-150410
- STATUS: RECOMMENDED
- TAGS: spike, modding, scenario, bundle

Refines and RE-PLANS the bundle family (supersedes the ordering in
`tasks/20260714-113418/SPIKE.md`). Two user goals reshape it: (1) content should be a
FOLDER bundle of files, not per-kind single files; (2) each thing's kind is a
`type`/`kind` FLAG INSIDE the RON structure, so one loader reads it and registers it
as section/scenario/ship/... - preparing the ground for a "real markup language" and
easy reuse. Plus: order the tasks so we do not hit the "fold" bump again (113408 built
a bespoke section catalog; then 113414's bespoke ship catalog had to be folded away).

## Question

How should the bundle family be structured and ORDERED so that: content is folder-
bundled; kind is a data flag (one generic loader + router, not one loader per
extension); the existing bespoke per-kind loaders collapse into the generic model
instead of accreting more; and the sequence builds the generic foundation FIRST so no
later kind needs a bespoke-then-fold detour?

## Context

Current model (what to change): nova_modding has TWO per-kind assets/loaders keyed by
EXTENSION - `ScenarioAsset` (`*.scenario.ron`) -> `GameScenarios`
(`HashMap<Id,ScenarioConfig>`), `SectionCatalogAsset` (`*.sections.ron`) ->
`GameSections` (`Vec<SectionConfig>`). `register_scenario`/`register_sections`
(nova_assets) each insert their registry at `Processing`. The 113418 spike recommended
CONTINUING this extension-typed pattern (option A). The user now wants the opposite:
kind INSIDE the file (option B), which the old spike rejected for reusing per-kind
loaders - but the user's markup-language + reuse goals make B the right call now.

Prototype-references + component-modifications already shipped for sections (113411);
ships are the folded 113414. Wasm constraint still holds (from 113418): no directory
enumeration on web, so a folder bundle needs a MANIFEST.

## Options considered

### How a file declares its kind

- **A. Extension-typed, per-kind loaders (current + old 113418 recommendation).** Keep
  `*.scenario.ron`/`*.sections.ron`/`*.ship.ron`, one loader each. Pro: already built.
  Con: kind is in the filename not the data; a new kind = a new loader + asset type;
  one file cannot mix kinds; NOT the markup-language shape the user wants. Rejected now.
- **B. Content-declared kind: a `Content` enum inside the RON (recommended).** One
  uniform content format - a RON `Vec<Content>` where
  `Content = Section(SectionConfig) | Ship(SpaceshipConfig) | Scenario(ScenarioConfig)
  | ...`. ONE `ContentLoader`; a generic `register_content` router dispatches each item
  by variant into its registry. A file can define sections, ships AND scenarios, each
  tagged. Pro: kind is data (the user's ask); one loader; new kind = one enum variant +
  one router arm; this IS the markup-language AST (a friendlier surface can lower to it
  later). Con: refactor the two existing per-kind loaders into it (re-touches shipped
  113408 code) - acceptable, and exactly "make the section catalog a case of the
  generic model."

### How a bundle is discovered

- **Manifest (recommended, unchanged from 113418).** A bundle is a directory + a
  `bundle.ron` listing its content files (relative paths). wasm-safe. `load_folder` is
  broken on web, so no directory enumeration.

### id / overlay - unchanged from 113418

- id-keyed registries + load-order overlay (base first, mods after, later id wins;
  intra-bundle dup = error).

## Recommendation

**Adopt B (content-declared `Content` kind) + folder bundles via manifest, and BUILD
THE GENERIC CONTENT MODEL FIRST so every kind is just a variant - no bespoke catalog to
fold.**

The shape:

- `Content` enum (kind flag): `Section(SectionConfig) | Ship(SpaceshipConfig) |
  Scenario(ScenarioConfig)` (grows by a variant). A content file is a RON `Vec<Content>`.
- One `ContentAsset(Vec<Content>)` + one `ContentLoader`. A generic
  `register_content(items, &mut registries)` routes each item by variant into
  `GameSections` / `GameShips` / `GameScenarios`. One arm per kind.
- A bundle = a directory + `bundle.ron` manifest listing content files; a bundle loader
  `load_context.load`s each content file and flattens the items; merge-by-kind with
  load-order overlay. Base game and a mod are both bundles.
- The existing `SectionCatalogAsset` + `ScenarioAsset` are REFACTORED INTO the `Content`
  model (kind moves from extension into data); the shipped section catalog becomes
  `[Section((..)), ...]` content. This is the migration, done as part of the foundation
  so nothing bespoke survives to fold.

### The re-ordering that avoids the fold

Do the GENERIC foundation first; every subsequent piece is "add a variant" or "package
into folders", never "build a bespoke catalog then fold it":

1. **Content model + generic router (FOUNDATION, first).** `Content` enum + `ContentLoader`
   + `register_content`; refactor the two existing per-kind loaders + the base game's
   loading + the committed RON (add the kind tag) onto it. After this, "a kind" is a
   variant, not a subsystem.
2. **Ship kind** (folds 113414): add `Content::Ship` + `GameShips` + `ShipSource`
   resolution + ship-modifications. Trivial on the foundation - a variant + a registry +
   spawn resolution.
3. **Folder bundle**: `bundle.ron` manifest + directory-of-content-files loading +
   merge-by-kind + overlay. (Single-file content already works from step 1; this adds
   the folder packaging.)
4. **Base game as a bundle**: the base content moves into `assets/base/` content files +
   a manifest, loaded through the bundle path.
5. **Mods + demo**: `mods.ron` index, load mod bundles after the base, overlay; a demo
   mod overriding a section and adding a scenario.

Why B beats A now: the user explicitly wants kind-in-data + folder bundles + a markup
foundation + reuse, and B delivers all four with one loader; A cannot mix kinds in a
file and grows a loader per kind. Why generic-first: the fold bump came from building a
bespoke catalog (113408) before the generic model existed; inverting the order means the
section catalog is just `Content::Section` and ships/scenarios/future kinds never need
their own subsystem.

## Open questions

- **`Content` file granularity.** `Vec<Content>` per file (a file may mix/group kinds)
  vs one `Content` per file. Recommend `Vec<Content>` (flexible; group by convention).
- **Registry shape unification.** `GameSections` is a `Vec`, `GameScenarios` a `HashMap`.
  For overlay-by-id, prefer id-keyed maps; decide whether to normalize `GameSections` to
  a map when planning step 1.
- **Migration blast radius.** Refactoring the shipped section/scenario loaders + RON is
  the cost of B; keep it behavior-preserving and parity-guarded (the existing parity
  tests carry over, re-generated in the Content shape).
- **Surface language later.** `Content` is the AST; a friendlier surface (KDL / a small
  DSL) that lowers to `Vec<Content>` revisits the parked option from spike
  20260708-161726 - out of scope here, but this is the groundwork.

## Next steps

Re-planned family (supersedes 113418's task ordering). A NEW foundation task fronts it;
the old 134xxx tasks are re-based on the Content model and re-gated:

- tatr 20260714-150508 (NEW, foundation): Content model + generic kind-router; refactor
  the existing per-kind loaders + base loading + RON onto it.
- tatr 20260714-134115 (re-based): ship kind as `Content::Ship` + GameShips + ShipSource.
- tatr 20260714-134119 (re-scoped): folder bundle - manifest + directory of content files.
- tatr 20260714-134123: base game as a bundle.
- tatr 20260714-134127: mods + overlay + demo.

## Fix record

(Appended by each implementing task as it lands.)
