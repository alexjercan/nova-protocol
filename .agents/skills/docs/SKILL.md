---
name: docs
description: Route and update Nova documentation, changelog, web content, and release documentation when behavior changes.
---

# Docs

Read `docs/keeping-docs-in-sync.md` and use its dependency map. Re-derive claims
from code; do not only grep for names.

- Put player behavior in `web/src/wiki/`, creator contracts in
  `web/src/create/`, and developer mechanisms in `docs/`.
- Ship invalidated documentation with the code change. Remove documentation for
  removed unshipped behavior instead of adding migration notes.
- Do not cite task artifacts in durable docs. `TODO(<task-id>)` is allowed for
  active work.
- Add one concise `CHANGELOG.md` entry for each released user-visible change.
  Use the last release as the baseline and mark format breaks `**(breaking)**`.
- Keep static fallback prose in every `data-widget` block. Source documented
  game numbers from Rust and record their `file:line` in a comment.
- Read `RELEASE.md` for a release; do not duplicate its checklist here.

Run the affected build and inspect its output:

```bash
nix develop --command mdbook build
cd web && npm run ci
```
