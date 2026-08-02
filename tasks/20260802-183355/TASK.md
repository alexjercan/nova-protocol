# Document nova_autopilot: rustdoc, prelude, and the dev wiki page

- PRIORITY: 93
- TAGS: v0.10.0, tooling, autopilot, docs
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183349

## Story

Document `nova_autopilot` as a crate a reader can adopt without reading its
source: crate-level ownership paragraph, the env contract table, the completion
protocol's two rules, doc examples on every plugin, a curated prelude, and a dev
wiki page the development docs link.

## Steps

- [ ] Complete the rustdoc pass: crate docs, per-module docs, compiling doc
      examples for each plugin, and prelude exports for every public item.
- [ ] Add the dev wiki page covering the env contract
      (`NOVA_AUTOPILOT`, `NOVA_SHOT`, `NOVA_REEL`, `NOVA_SHOT_DIR`,
      `NOVA_AUTOPILOT_DEADLINE`) and register it in the docs routing map,
      `web/webpack.config.js`, and `web/src/wiki-pages.ts`.

## Definition of Done

- Rustdoc builds warning-free with `missing_docs` on.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)
- Doc examples compile.
  (cmd: `nix develop --command cargo test --doc -p nova_autopilot`)
- The wiki page is routed and the website build passes.
  (cmd: `cd web && npm run ci`)
- Every public item is reachable through the prelude.
  (manual: read the prelude against the crate's public surface)

## Notes

- Parent: `20260802-120019`. Depends on the driver ports.
- Routing map: `web/src/wiki/dev/keeping-docs-in-sync.md`.
