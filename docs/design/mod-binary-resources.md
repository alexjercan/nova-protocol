# Mods ship their own binary resources (mod-relative asset refs)

Task 20260716-123544 (pipeline half). Lets a mod bundle carry its own binary
assets (PNG textures, skyboxes + their `.meta` sidecars, GLB models, audio) and
reference them from its content with mod-relative asset paths, resolved against
the OWNING mod's folder - shipped (`assets/mods/<id>/`) or downloaded
(`mods://<id>/`), native and web.

## The gap this closes

Before this change, every `AssetRef` path in any mod's content resolved against
the BASE game's asset root. Gauntlet's skybox was `"textures/cubemap.png"` - a
base file. A mod could not point at a texture it shipped itself; a bundle
manifest listed only `content` RON files.

## Author-facing contract

- A bundle manifest gains a `resources: Vec<String>` list (default empty):
  the binary files the bundle ships, as paths RELATIVE to the bundle directory
  (same base as `content` and `meta.icon`). Strict-RON string list:
  `resources: ["textures/nebula.png", "models/hull.glb"]`.
- Content references a shipped resource with the reserved `self://` scheme in
  any `AssetRef` string: `cubemap: "self://textures/nebula.png"`,
  `texture: "self://textures/rock.png"`, `render_mesh: Some("self://models/hull.glb#Scene0")`.
  `self://` means "this mod's own folder". (At the time, a bare path with no
  scheme still resolved against the base game. **SUPERSEDED by Option A**: bare
  asset refs are retired - every ref is now namespaced, and a base asset is
  `dep://base/<path>`. See the Option A section below.)
- Sidecar `.meta` files (e.g. a skybox's `<name>.png.meta` with
  `array_layout: RowCount`) are shipped as ordinary files in the bundle folder;
  they ride along automatically and do not need a `self://` ref or a `resources`
  entry - bevy loads `<asset>.meta` next to the asset on both targets.
- `self://` is a RESERVED PREFIX for ALL string content in a mod, not only asset
  paths. The rewrite/validation walk is generic (every string leaf), so a
  free-text field - a `DebugMessage`, a `StoryMessage` line, an objective
  message, a `VariableSet` key, a string `VariableLiteral` - that begins with the
  literal `self://` would be rewritten and gated as if it were a resource ref. In
  practice no message legitimately starts with `self://`; authors just avoid it
  as a leading token. (A `self://` mid-string is untouched - only a leading
  match rewrites.)

## Resolution mechanism (rewrite at flatten)

`self://` is a SENTINEL, never a real bevy asset source. It is rewritten to a
concrete asset path before it reaches the `AssetServer`, at bundle-merge time
(`register_bundles`), where the owning mod is known:

- The `BundleAssetLoader` computes each bundle's `resource_base` from its own
  load path (`load_context`): the bundle DIRECTORY as an asset-path string with
  the source scheme preserved - `mods/<id>` for a shipped bundle (default
  source), `mods://<id>` for a downloaded one. It is carried on `BundleAsset`
  alongside the manifest's `resources`.
- `register_bundles` rewrites every content item before merging: each
  `self://X` leaf becomes `<resource_base>/X`. Shipped -> `mods/<id>/X` (default
  source -> `assets/mods/<id>/X`); downloaded -> `mods://<id>/X` (the existing
  sandboxed cache source, native dir + web IndexedDB). The rewritten path is a
  normal asset path, so `AssetRef::resolve`, the skybox loader, GLB loading and
  the web target all work with zero further changes.

### Why rewrite, and why generically

`AssetRef` is buried across many content structs (skybox cubemap + thumbnail,
asteroid texture, every section render mesh, muzzle effects). A per-field
visitor would silently miss any field added later. `Content` and its trees do
not derive `Reflect` (and `AssetRef` is hand-serialized), so a reflection walk
is unavailable. But every content type derives `Serialize + Deserialize` and
`AssetRef` serializes as a bare string, and `SectionId = String` so the tree has
no non-string map keys. So the rewrite serializes a `Content` to a
`serde_json::Value`, rewrites every string leaf that starts with `self://`, and
deserializes back - catching every `AssetRef` field with zero per-field code and
no maintenance as new fields appear. The round-trip is lossless (all-string map
keys; f32 <-> f64 is exact) and mirrors the `content_ron_parity` guarantee the
codebase already relies on. If (de)serialization ever fails it logs and keeps
the original item rather than panicking - bad data cannot crash the app.

### Alternatives rejected

- Resolve at `AssetRef::resolve` time with a per-mod prefix: `resolve` is called
  from many action/spawn sites operating on MERGED content that has lost the
  owning-mod identity; threading context to every call site is invasive.
- Per-mod asset sources: bevy asset sources are registered at App build, not
  dynamically per enabled mod at runtime.
- Rewrite in the content loader (relative to the content file): the content file
  may sit in a subdir, so its dir differs from the bundle dir; that would make
  `self://` relative to the file while `resources`/`meta.icon` are relative to
  the bundle - two bases. Bundle-dir-relative everywhere is the one rule.

## Validation (membership, in every domain it crosses)

Following the `validate-in-every-domain` lesson, a `self://` ref that does not
name a declared `resources` member is rejected wherever the data is checked:

- Portal generator (`nova_portal_gen`): two memberships. Every `resources` entry
  must be a MEMBER of the walked+hashed file set (same check as `content`) -
  rejects a declared resource that is missing, escaping (`../`), or not
  slash-normalized. AND every `self://` ref in the content must name a declared
  resource - the content is parsed to a `ron::Value` (comments stripped, so a
  `self://` in a comment is not a false hit) and each `self://` string leaf is
  checked. This closes the publish-time hole for a mod published from OUTSIDE the
  repo tree (which the static lint below never sees). The generator stays
  engine-free - `ron::Value`, no bevy, no typed `Content`. Files distribute
  automatically (it copies every file verbatim with size+sha256).
- Static lint walk (`lint_walk`, the `content_lint` bin + CI gate): every
  `self://` ref in any bundle's content - SECTION or scenario - must name a
  declared `resources` member of that bundle. Covers base, `assets/mods/*` and
  `webmods/*`.
- Runtime (`register_bundles`): same check on the merged/enabled set. An
  undeclared `self://` ref in a SCENARIO is recorded as an Error content issue,
  so the runtime content gate refuses to start that scenario. A SECTION's
  undeclared ref is logged (the runtime gate is scenario-scoped by design, task
  20260716-193949 - `ContentIssues` is keyed by scenario id); it does not block,
  but the same ref is already caught before it ships by the static lint (repo
  mods) and the portal generator (published mods), and at spawn the rewritten
  path fails to load loudly. Extending the runtime gate to sections is left to
  the gate's own follow-up rather than special-cased here.

## Cross-mod references (`dep://<id>/`, task 20260716-215423)

`self://` is deliberately own-folder only. Task 20260716-215423 adds a second
sentinel, `dep://<id>/<path>`, so a mod can reference a file shipped by a
DECLARED DEPENDENCY `<id>` - enabling a shared "art pack" several mods depend on
for a common look without each copying the bytes.

### Scheme choice: `dep://`, not `mod://`

The obvious `mod://<id>/` was rejected: the codebase already uses `mods://` (WITH
an `s`) as a LIVE bevy asset SOURCE for downloaded bundles, so `mod://` is one
keystroke from a real source - a sentinel a typo away from silently loading real
bytes. `dep://` is unambiguous and states the gate (the target must be a declared
DEPendency). It pairs with `self://`: `self://X` = my own folder, `dep://<id>/X`
= dependency `<id>`'s folder.

`dep://base/...` was REJECTED at first (in this task): `base` was an implicit
dependency whose files were at the asset ROOT (referenced with a bare path),
while base's `resource_base` was its own folder (`base`), so
`dep://base/textures/x` would have wrongly pointed at `assets/base/textures/x`
instead of the root `assets/textures/x`. **SUPERSEDED by Option A** (see below):
base art was MOVED under `assets/base/`, which makes `resource_base = "base"`
correct, and `dep://base/...` is now the canonical way to reference base art.

### Resolution (same rewrite-at-flatten)

`register_bundles` merges bundles in dependency-topological order, so a
dependency's `BundleAsset` (its `resource_base` + `resources`) is already loaded
when a dependent is flattened. During the flatten, each owning bundle gets a
`mod_refs::RefScope` carrying its own `resource_base`/`resources`, its declared
dependency ids, and - for the declared deps that are enabled+loaded - their
`resource_base`/`resources`. Then:

- `self://X`     -> `<own resource_base>/X`   (unchanged).
- `dep://<id>/X` -> `<dep's resource_base>/X` when `<id>` is a declared, available
  dependency (shipped -> `mods/<id>/X`, downloaded -> `mods://<id>/X`); otherwise
  LEFT LITERAL (an unknown `dep` source that fails to load loudly, exactly as an
  undeclared `self://` resolves-but-404s), with the violation recorded for the
  gate.

The rewrite stays the generic serde-value string-leaf walk; the two schemes share
one `RefScope`, and the typed-`Content` helpers (`rewrite_refs`,
`resource_ref_violations`) are used by BOTH the runtime and the static lint.

### Validation (both halves, in every domain)

A `dep://<id>/X` ref is valid iff (a) `<id>` is a declared dependency of the
referencing bundle (and not `base`), AND (b) `X` is a declared `resources`
member of dependency `<id>`. Enforced everywhere `self://` is:

- Portal generator (engine-free `ron::Value`): `build_entry` checks half (a)
  locally against the mod's own `meta.dependencies` (and rejects `dep://base` and
  malformed `dep://` leaves); `generate` checks half (b) across all portal
  entries, where every portal mod's `resources` are known. When `<id>` is a
  SHIPPED dependency the portal knows only its id (the shipped catalog is a thin
  id list), so half (b) is skipped there - backstopped by the runtime gate and
  the repo lint. Half (a) still holds via the existing dependency-resolution
  check.
- Static lint walk (`lint_walk`): both halves over the repo tree; the deps'
  `resources` come from the walked set.
- Runtime (`register_bundles`): both halves on the merged/enabled set; an
  undeclared/ungated `dep://` ref in a SCENARIO is an Error content issue so the
  gate refuses the scenario (sections logged, matching the `self://` policy).

A declared-but-unavailable dependency (not installed, or a downloaded dep still
loading) yields a "not available" violation and a literal ref; the loaded-event
re-run of `register_bundles` fixes the transient download case, as for `self://`.

### Shipped dogfood: a follow-up

Shipping an actual art-pack + consumer pair ripples through installed-count
assertions and wants real art, so it is a separate CONTENT task. This task
proves the pipeline end-to-end with SYNTHETIC bundles (the cross-mod cases in
`mod_binary_resources.rs`) plus unit tests, mirroring the `self://` gate test.

## Option A: base as a normal mod / canonical scheme model (tasks 20260717-000416, -002105, -002133)

The user chose to take the "base is just a mod" principle all the way, Factorio
style, retiring the special "bare path == base game" convention. Spike:
`tasks/20260716-235458/SPIKE.md` (Option A chosen over the spike's Option B
recommendation, for canonical one-way-to-reference uniformity while the mod
ecosystem is small). Three landed tasks:

- **`dep://base` mechanism (20260717-000416).** `base` is the IMPLICIT UNIVERSAL
  dependency: `dep://base/<path>` is allowed without base appearing in a mod's
  `meta.dependencies`. `register_bundles` and the static lint inject `base` into
  every owning bundle's dep scope (its `resource_base` + `resources`); the portal
  exempts `base` from the declared-dep check (membership skipped for base as a
  shipped dep, backstopped by the repo lint + runtime). The earlier `dep://base`
  REJECTION is gone.
- **The migration (20260717-002105).** Base art (`gltf`, `textures` + `.meta`,
  `banner.png`) MOVED from the asset root to `assets/base/`, so base is a
  self-contained bundle. `GameAssets` `#[asset(path)]` + `meta_check` repointed to
  `base/...`; `gen_content` builders emit `self://` and base declares a
  `resources` list; every shipped/web mod's bare base ref became `dep://base/...`.
  `icons/` (game UI) and `shaders/` (engine) stay at root. Now
  `resource_base = "base"` is correct: base's `self://X` and any `dep://base/X`
  both resolve to `assets/base/X`.
- **Sounds under base + section-authored audio (20260717-002228).** The
  `sounds/` follow-up: base `sounds/` MOVED to `assets/base/sounds/` and joined
  the base `resources` list, so mods can reference them with
  `dep://base/sounds/<name>.wav`. `register_sounds` loads them with
  `SoundBank::load_paths` from `base/sounds/<name>.wav` (the global bank cues -
  damage/UI/thruster - stay code-driven). Sections can now DECLARE a sound as an
  authorable `AssetRef<AudioSource>` content field, exactly like a render mesh:
  the first is the turret section's `fire_sound`, snapshotted (unresolved) onto
  the turret at spawn and resolved + preferred over the bank cue by the audio
  observer, so a mod turret ships + references its own weapon sound through the
  same `self://`/`dep://` walk. Audio was the LAST
  root-art exception; nothing base loads sits at the asset root now except
  `icons/`/`shaders/`.
- **Canonical enforcement (20260717-002133).** An asset ref in content MUST carry
  a scheme (`self://` / `dep://`). A bare, scheme-less asset-path ref is an Error
  at author/publish time (static `content_lint` + portal). The hard no-bare
  guarantee is STRUCTURAL - a bare ref resolves against the default source and
  404s at load (base art is no longer at the root). The lint uses an
  asset-EXTENSION heuristic (a scheme-less `.png`/`.glb`/... string is a forgotten
  scheme): the generic content walk cannot type-distinguish an `AssetRef` from any
  other string, and `AssetRef`'s dual authoring/resolved role rules out a
  deserialize-level ban (the merge rewrite round-trips bare RESOLVED paths through
  it). No runtime gate (user decision).

Net author-facing model: three schemes, one canonical way, no bare paths -
`self://` (own folder), `dep://<id>/` (a declared dependency), `dep://base/` (the
base game, implicit).

## Dogfood

The shipped `example` mod (`assets/mods/example/`) carries placeholder binaries -
a `nebula.png` skybox (+ `.meta`) and a `rock.png` asteroid texture - referenced
via `self://`, proving a scenario renders from mod-owned assets, not base
`assets/`. (The dogfood originally shipped as a standalone `variety` mod; it was
folded into the single `example` tutorial mod under task 20260716-215513.) The
real art replaces the placeholders under task 20260716-205214.

## Implementation notes & reflection

- The crux was the rewrite MECHANISM, not the manifest field. `AssetRef` is
  buried across many content structs and the tree does not derive `Reflect`, so
  the first instinct (a per-field or reflection visitor) was a dead end. The
  breakthrough was noticing every content type derives `Serialize`/`Deserialize`
  and `AssetRef` serializes as a bare string, and `SectionId = String` leaves the
  tree with no non-string map keys - so a serde-value round-trip is a safe,
  generic, zero-maintenance visitor. This was de-risked empirically (round-trip
  all 10 committed content files) before trusting it.
- Adding a shipped mod broke a spread of hardcoded "base + demo = 2 installed"
  count assertions across `demo_scenario.rs` and `mod_cache_install.rs`
  (including downloaded rows shifting from index 2 to 3). A full, untruncated
  sweep for count assertions BEFORE re-running caught them together, rather than
  one red run at a time (the `truncated-sweep-is-not-a-sweep` lesson).
- A blunt `!contains("self://")` test assertion tripped on the word `self://`
  inside a `DebugMessage` string - the same reserved-prefix collision the design
  now documents. Surfaced the footgun early; the shipped content and the test
  were both cleaned up.
- The adversarial review's most valuable catch was the portal generator gap: it
  validated that declared resources exist as files but never that content refs
  name a declared resource. `ron::Value` (comment-stripping, engine-free) closed
  it without pulling bevy into the generator. Worth doing even though the in-repo
  static lint already covered every mod THIS project publishes.
- Friction, for next time: this ran in a background job pinned to one worktree,
  so `sprout`'s cache-dir worktree was unusable and edits had to go through a
  `.claude/worktrees/` worktree created off local master (origin/master was 69
  commits behind, so the default `fresh` base would have been stale). Check the
  isolation model before sprouting when running as a background job.
