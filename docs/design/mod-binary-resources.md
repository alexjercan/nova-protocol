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
  `self://` means "this mod's own folder". A bare path (no scheme) still
  resolves against the base game, so existing base-relative refs are unchanged.
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

## Dogfood

A shipped `variety` mod (`assets/mods/variety/`) carries placeholder binaries -
a `nebula.png` skybox (+ `.meta`) and a `rock.png` asteroid texture - referenced
via `self://`, proving a scenario renders from mod-owned assets, not base
`assets/`. The real art replaces the placeholders under task 20260716-205214.

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
