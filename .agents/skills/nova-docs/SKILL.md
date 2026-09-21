---
name: nova-docs
description: Update Nova docs, wiki, website, changelog, and release material.
---

# Nova Docs

Read the [dependency map](../../../docs/keeping-docs-in-sync.md). Re-derive
claims from code; do not update pages from name searches alone.

## Route the change

- Put player behavior in `web/src/wiki/`.
- Put creator and format contracts in `web/src/create/`.
- Put developer mechanisms in `docs/` and API details in rustdoc.
- Put site implementation in `web/` and release stories in `web/src/news/`.
- Read `RELEASE.md` for a release. Do not duplicate its checklist here.

## Write the current truth

- Ship invalidated docs with code. Delete docs for removed unshipped behavior;
  do not write migration notes for it.
- Do not cite completed tasks. Use `TODO(<task-id>)` only for active work.
- Apply the Changelog section of [AGENTS.md](../../../AGENTS.md) to every
  `CHANGELOG.md` edit. It owns those rules; do not restate them here.
- Keep static prose in each `data-widget` block. Source game numbers from Rust
  and record their `file:line` in a source comment.
- Do not add tests for prose. Check links, manifests, generated pages, and the
  behavior of code-backed widgets when those are the changed contract.

Run the affected build and inspect its output:

```bash
nix develop --command mdbook build
cd web && npm run ci
```

Inspect layouts, links, images, and responsive behavior for frontend changes.
A docs-only edit does not require a game run. A visual claim requires rendered
output; a successful headless build is not visual evidence.
