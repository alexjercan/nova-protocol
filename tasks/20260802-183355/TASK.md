# Document nova_autopilot: rustdoc, prelude, and the dev wiki page

- PRIORITY: 93
- TAGS: v0.10.0, tooling, autopilot, docs
- KIND: TASK
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183349

## Story

Document `nova_autopilot` as a crate a reader can adopt without reading its
source: crate-level ownership paragraph, the env contract table, the completion
protocol's two rules, doc examples on every plugin, a curated prelude, and a dev
wiki page the development docs link.

The four driver modules landed one per task, each documenting itself. What is
missing is the crate-level view a first-time reader needs: `pub mod prelude {}`
is still literally empty (`crates/nova_autopilot/src/lib.rs:32`), `completion.rs`
carries no code fence at all, and the env contract is spread across four
`*_ENV` consts in four modules with no single table. This task closes that and
clears the two review nits each of the last two ports deferred here.

## Steps

- [ ] Fill `prelude` in `crates/nova_autopilot/src/lib.rs` with every public item
      of the four modules, and add `crates/nova_autopilot/tests/prelude.rs`: a
      compile-time test that names each one through `use
      nova_autopilot::prelude::*` and asserts the env consts, so a new public
      item that skips the prelude fails the build rather than a reading.
- [ ] Extend the crate docs in `lib.rs` with (a) the env contract table -
      `NOVA_AUTOPILOT`, `NOVA_SHOT`, `NOVA_REEL`, `NOVA_SHOT_DIR`,
      `NOVA_AUTOPILOT_DEADLINE` - each with what arms it and which plugin reads
      it, (b) the completion protocol's two rules (register before the run
      starts; the app exits only when every registrant reports done), and (c) a
      pointer to `examples/driven_app.rs` as the end-to-end read
      (`20260802-183352` nit).
- [ ] Add a compiling doc example to `completion.rs` - the only module with
      none - showing `register` + `HarnessCompletion::done` for a caller-owned
      collector.
- [ ] Clear the `20260802-183349` nits in `crates/nova_autopilot/src/reel.rs`:
      say on `ScreenshotReelPlugin::ready` that the predicate is re-evaluated
      every frame until it returns `true`, and make the `capture_path` unit
      test set/clear `NOVA_SHOT_DIR` itself (or assert both branches) instead of
      skipping when the env is ambient.
- [ ] Add `web/src/wiki/dev/automation-harness.md`: what the crate drives, the
      env contract table, the completion protocol, and how a Nova example opts
      in. Register the slug `dev/automation-harness` in
      `web/webpack.config.js` (`WIKI_DOC_PAGES`), `web/src/wiki-pages.ts`
      (slug, title, category, tags, summary, related, headings), and add its row
      to the dependency map in `web/src/wiki/dev/keeping-docs-in-sync.md`.

## Definition of Done

- Every public item is reachable through the prelude, enforced by a test rather
  than a reading. (test: `prelude_names_every_public_item`)
- The completion protocol carries a compiling doc example, and every doc example
  in the crate still compiles.
  (cmd: `nix develop --command cargo test --doc -p nova_autopilot && rg -n '^/// ```|^//! ```' crates/nova_autopilot/src/completion.rs`)
- The dev wiki page exists and is routed through all three registries.
  (cmd: `test -f web/src/wiki/dev/automation-harness.md && rg -n 'dev/automation-harness' web/webpack.config.js web/src/wiki-pages.ts web/src/wiki/dev/keeping-docs-in-sync.md`)
- The website build still passes with the new page.
  (cmd: `cd web && npm run ci`)
- Rustdoc builds warning-free with `missing_docs` on.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Notes

- Parent: `20260802-120019`. Depends on the driver ports.
- Routing map: `web/src/wiki/dev/keeping-docs-in-sync.md`. Page shape to copy:
  `dev/keeping-docs-in-sync` in `web/webpack.config.js:96-105` and
  `web/src/wiki-pages.ts:436-451`.
- Base-branch proof status, checked 2026-08-03: the prelude test file, the
  `completion.rs` fence and the wiki page are all absent, so the first three DoD
  proofs are red. `cargo doc -Dwarnings` and `cargo test --doc` already pass on
  base (3 compile-only doc tests) - they are regression guards, which is why the
  doc-example criterion chains the fence check that is actually red.
- The wiki page documents the crate's own `NOVA_*` contract, which is already
  true on base. Nova's own callers still read `BCS_*`; renaming them is
  `20260802-183403` and sweeping `dev/development.md` is `20260802-183406`.
  Do not touch those files here.
- Public surface to cover in the prelude: `autopilot::{AutopilotPlugin,
  AutopilotLoop, AUTOPILOT_ENV}`, `completion::{HarnessCompletion, register,
  AUTOPILOT, SCREENSHOT, REEL, DEADLINE_ENV, DEFAULT_DEADLINE_SECS}`,
  `screenshot::{ScreenshotPlugin, SCREENSHOT_ENV, MAX_WAIT_FRAMES}`,
  `reel::{ScreenshotReelPlugin, ReelBeat, capture_window, REEL_ENV,
  SHOT_DIR_ENV, REEL_CAPTURE_RESOLUTION}`. Confirm against `rg -n '^pub '` at
  build time rather than trusting this list.
- Assumption: the prelude re-exports names verbatim. `ScreenshotPlugin` clashes
  with Bevy's own under a glob import, but this crate's prelude is not glob-
  imported next to `bevy::prelude::*` by any current caller (`nova_debug` keeps
  its own curated prelude for exactly that reason,
  `crates/nova_debug/src/lib.rs:34`). If `20260802-183403` hits the clash, it
  aliases at its own import site, not here.
