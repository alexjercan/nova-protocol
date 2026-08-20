# Nova Protocol docs/

`docs/` is the SOURCE of the developer book - an mdbook configured by
`book.toml` at the repo root and published at
`https://alexjercan.github.io/nova-protocol/dev/` by the pages deploy.

- Build: `nix develop --command mdbook build` (output in `book/`, gitignored).
- Chapters are listed in `SUMMARY.md`; `mermaid` fences render via
  mdbook-mermaid (assets installed at the repo root).
- Start at `introduction.md`; the documentation map is
  `keeping-docs-in-sync.md`.

`docs/` is NOT a scratchpad: everything here is a durable, maintained book
chapter. Transient working files live outside the repo; task-scoped records
live in `tasks/<id>/`. `keeping-docs-in-sync.md` owns the rest of that rule.
