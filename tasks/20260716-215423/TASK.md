# Cross-mod resource refs: reference a declared dependency's shipped resources

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.7.0,modding,feature,assets,spike


## Gap (surfaced by user 2026-07-16)

The mod-binary-resources feature (task 20260716-123544, landed) added `self://`
asset refs, but they are deliberately scoped to the OWNING mod's own folder:
`self://X` rewrites to that bundle's `resource_base` and nothing else
(crates/nova_modding/src/lib.rs, `resource_base`). So a mod CANNOT reference
another mod's resources. Today the only robust option is to bundle your own
copy; a bare hardcoded `mods/<otherid>/...` path happens to work for a SHIPPED
dependency but breaks for a DOWNLOADED one (it lives at `mods://<otherid>/...`)
and is not validated by any gate - a footgun, not a feature.

## Goal

Let a mod reference a DECLARED DEPENDENCY's resources with the same
shipped-vs-downloaded transparency `self://` gives, so shared art/audio packs
are possible without every mod copying the bytes.

## Spike (design decision first - hence the spike tag)

- Scheme shape: e.g. `mod://<id>/<path>` (or `dep://<id>/...`) that resolves the
  named mod's folder, gated on `<id>` being a DECLARED dependency of the
  referencing bundle (reuse the existing `meta.dependencies` + dep resolver).
  Reject a ref to a non-dependency (a mod must not reach into an arbitrary
  other mod).
- Resolution: reuse the merge-time rewrite. At `register_bundles` the owning
  bundle's `resource_base` is known; the DEPENDENCY's `resource_base` is also
  known (it is an enabled/loaded bundle). Rewrite `mod://<id>/X` to that
  dependency's `resource_base`/X - shipped `mods/<id>/` or downloaded
  `mods://<id>/`, native + web, same as `self://`.
- Validation (validate-in-every-domain): the referenced file must be a declared
  `resources` member of the DEPENDENCY, and `<id>` must be a declared
  dependency. Enforce in the portal generator, the static content_lint, and the
  runtime gate - mirroring the `self://` checks.
- Ordering/availability: a dependency merges before its dependents (topological
  order already guarantees this), so the dep's bundle/resources are known when
  the dependent is rewritten. Confirm for the download case too.

## Consumers / motivation

- A shared "art pack" mod that several scenario/campaign mods depend on for a
  common look (skyboxes, textures) without each shipping duplicate bytes.

## Documentation (do as part of this)

- The mod-authoring guide + design doc currently document `self://` (own folder)
  but are SILENT on cross-mod use. When this lands, document the `mod://<id>/`
  scheme AND, until then, the limitation (self:// is own-folder only). Update
  web/src/wiki/dev/guide-make-a-mod.md, modding-ron.md, and
  docs/design/mod-binary-resources.md.

## Notes

- Depends on the landed self:// pipeline (docs/design/mod-binary-resources.md).
