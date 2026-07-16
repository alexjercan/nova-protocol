# Canonical enforcement: lint rejects bare asset refs in content (Option A)

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.7.0,modding,lint,feature

## Goal

Make the namespaced scheme model CANONICAL: an asset reference authored in
content must carry a scheme (`self://` or `dep://<id>/`). A bare, scheme-less
asset-path ref is rejected at AUTHOR time with a clear message, instead of
silently 404-ing at load (which is what happens structurally after task
002105 moves base art off the root).

## Design constraint (from the spike + code)

The generic string-leaf walk (mod_refs) CANNOT tell an asset path from a message
string, and `AssetRef::deserialize` cannot hard-reject schemeless strings because
the merge-rewrite round-trips through serde and PRODUCES schemeless resolved
paths (`self://x` -> `base/x`) that must deserialize back. So the ban must be a
TYPED check over `AssetRef` fields on the AUTHORED (pre-rewrite) content, at the
static layer - NOT the runtime gate, NOT `AssetRef::deserialize`.

## Steps

- [ ] Decide the typed-visitor mechanism: how to enumerate the `AssetRef` string
      fields of a `Content` item for validation without the generic walk (a small
      targeted visitor, or a serde-based pass that only inspects fields known to
      be asset refs). Confirm it does NOT false-positive on non-asset strings
      (section ids, scenario ids, messages, variable names).
- [ ] `content_lint` (static `lint_walk`): flag every bare asset-path ref in any
      bundle's content with an Error - "asset ref '<path>' has no scheme; use
      'self://<path>' or 'dep://<id>/<path>'".
- [ ] `nova_portal_gen` (engine-free `ron::Value`): mirror the check at publish.
- [ ] Do NOT add a runtime gate (user decision) - runtime keeps working via the
      rewrite; bare simply fails to load. Document that the enforcement is
      static (author/publish time).
- [ ] Tests: a bare asset ref in content is a lint error (static + portal); a
      schemed ref passes; a non-asset string that happens to look path-like is
      NOT flagged.
- [ ] Ensure the repo tree passes the new gate (all base + mod content already
      migrated to schemes by task 002105).

## Notes

- Depends on task 20260717-002105 (migration - the tree must be scheme-clean
  before this gate turns on, or CI goes red).
- Spike: tasks/20260716-235458/SPIKE.md (Option A, item 5).
