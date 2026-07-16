# Spike: cross-mod resource refs (reference a declared dependency's resources)

Task 20260716-215423. Resolves the design fork before implementation. Builds
directly on the landed `self://` pipeline (docs/design/mod-binary-resources.md).

## Question

A mod can ship its own binaries and reference them with `self://X` (own folder
only). It CANNOT reference another mod's shipped resources. We want a shared
"art pack" mod that several mods depend on for a common look, without every mod
copying the bytes. What scheme, gated how, resolved where?

## Decision

### Scheme: `dep://<id>/<path>` (NOT `mod://`)

`dep://<id>/<path>` references file `<path>` shipped by mod `<id>`, where `<id>`
MUST be a declared dependency (`meta.dependencies`) of the referencing bundle.

Chosen over the task's alternative `mod://<id>/` because the codebase already
uses `mods://` (WITH an `s`) as a LIVE bevy asset SOURCE for downloaded bundles.
`mod://` (no `s`) is one keystroke from `mods://`: a sentinel one typo away from
a real source is a footgun. `dep://` is unambiguous AND states the gate - the
target must be a declared DEPendency. It pairs cleanly with `self://`:

- `self://X`      -> a file in MY OWN folder.
- `dep://<id>/X`  -> a file in dependency `<id>`'s folder.

Like `self://`, `dep://` is a SENTINEL, never a real asset source: it is always
rewritten away before the path reaches the `AssetServer`.

`dep://base/...` is REJECTED. `base` is an implicit dependency whose files are
referenced with a bare (root-relative) path already; base's `resource_base` is
its own folder (`base`), so rewriting `dep://base/textures/x` would wrongly
point at `assets/base/textures/x` instead of the root `assets/textures/x`. Bare
paths are the one correct way to reach base.

### Resolution: reuse the merge-time rewrite

At `register_bundles`, bundles merge in dependency-topological order, so a
dependency's `BundleAsset` (with its `resource_base` and `resources`) is already
loaded when a dependent is flattened. During the existing per-bundle flatten:

- `self://X`     -> `<own resource_base>/X`   (unchanged)
- `dep://<id>/X` -> `<dep's resource_base>/X` when `<id>` is a declared,
  available dependency; otherwise LEFT LITERAL (fails to load loudly, like an
  undeclared `self://`), with the violation recorded for the content gate.

Shipped dep -> `mods/<id>/X`; downloaded dep -> `mods://<id>/X`; native + web,
identical to `self://`. The rewrite stays the generic serde-value string-leaf
walk (zero per-field code), extended to the second scheme.

### Validation (validate-in-every-domain, mirroring `self://`)

A `dep://<id>/X` ref is valid iff: (a) `<id>` is a declared dependency of the
referencing bundle (and not `base`), AND (b) `X` is a declared `resources`
member of dependency `<id>`. Enforced in all three domains:

- Portal generator (`nova_portal_gen`, engine-free `ron::Value`): `<id>` must be
  in the mod's own `meta.dependencies` (local check); if `<id>` is another
  PORTAL mod, `X` must be in its `resources` (cross-mod check in `generate`,
  where all entries are known). If `<id>` is SHIPPED (only ids known to the
  portal), the membership half is skipped - backstopped by the runtime gate and
  the repo lint. The existing dependency-resolution check already guarantees a
  declared dep resolves within the portal+shipped set.
- Static lint walk (`lint_walk`): over the repo tree, `<id>` must be a declared
  dep and `X` a member of that walked bundle's `resources`.
- Runtime (`register_bundles`): same check on the merged/enabled set; an
  undeclared/ungated `dep://` ref in a SCENARIO is an Error content issue so the
  gate refuses the scenario (sections logged, matching the `self://` policy).

### Ordering / availability

Topological order already guarantees a dependency merges before its dependents,
and enabling a mod auto-enables its (transitive) dependencies, so a declared
dep's bundle is normally loaded when the dependent is rewritten. A
declared-but-unavailable dep (not installed, or a downloaded dep still loading)
yields a "not available" violation and a literal ref; the loaded-event re-run of
`register_bundles` fixes the transient download case, exactly as for `self://`.

## Shared implementation

The typed-`Content` helpers (`mod_refs.rs`, used by runtime + static lint)
unify both schemes behind one `RefScope` (own base/resources + declared deps +
available deps' base/resources) with two passes: `resource_ref_violations`
(replaces `undeclared_self_refs`) and `rewrite_refs` (replaces
`rewrite_self_refs`). The portal generator mirrors the classification on
`ron::Value` (engine-free), as it already does for `self://`.

## Seeded follow-up

Shipping an actual "art pack" dogfood (a shared-resource mod + a consumer mod
that `dep://`-references it) ripples through installed-count assertions and
wants real art; it is a CONTENT task, seeded separately, not part of this
mechanism task. This task proves the pipeline end-to-end with synthetic bundles
(mirroring the existing `self://` gate test) plus unit tests.
