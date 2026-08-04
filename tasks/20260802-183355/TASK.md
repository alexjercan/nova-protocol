# Document nova_autopilot: rustdoc, prelude, and the dev wiki page

- PRIORITY: 93
- TAGS: v0.10.0, tooling, autopilot, docs
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
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

- [x] Fill `prelude` in `crates/nova_autopilot/src/lib.rs` with every public item
      of the four modules, and add `crates/nova_autopilot/tests/prelude.rs`: a
      compile-time test that names each one through `use
      nova_autopilot::prelude::*` and asserts the env consts, so a new public
      item that skips the prelude fails the build rather than a reading.
- [x] Extend the crate docs in `lib.rs` with (a) the env contract table -
      `NOVA_AUTOPILOT`, `NOVA_SHOT`, `NOVA_REEL`, `NOVA_SHOT_DIR`,
      `NOVA_AUTOPILOT_DEADLINE` - each with what arms it and which plugin reads
      it, (b) the completion protocol's two rules (register before the run
      starts; the app exits only when every registrant reports done), and (c) a
      pointer to `examples/driven_app.rs` as the end-to-end read
      (`20260802-183352` nit).
- [x] Add a compiling doc example to `completion.rs` - the only module with
      none - showing `register` + `HarnessCompletion::done` for a caller-owned
      collector.
- [x] Clear the `20260802-183349` nits in `crates/nova_autopilot/src/reel.rs`:
      say on `ScreenshotReelPlugin::ready` that the predicate is re-evaluated
      every frame until it returns `true`, and make the `capture_path` unit
      test set/clear `NOVA_SHOT_DIR` itself (or assert both branches) instead of
      skipping when the env is ambient.
- [x] Add `web/src/wiki/dev/automation-harness.md`: what the crate drives, the
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

## Close-out

What and why. `nova_autopilot` now reads as a crate rather than four modules:
`prelude` re-exports all 19 public items verbatim, `lib.rs` carries the env
contract table, the completion protocol's two rules and a pointer to
`examples/driven_app.rs`, `completion.rs` gained the caller-owned-collector doc
example it was missing, and `web/src/wiki/dev/automation-harness.md` is the
contributor-facing version of the same three things, routed through all three
registries. The two deferred review nits are cleared in `reel.rs`.

Enforcement over review. `tests/prelude.rs` is not just a compile-time naming
of the re-exports (which only catches a DELETED export). It also `include_str!`s
the four module sources, scans column-0 `pub ` items, and asserts each one is in
the prelude list - so a NEW public item that skips the prelude fails the build.
An unrecognized item kind panics rather than being skipped, so the scan cannot
quietly stop covering the surface.

Alternatives. (a) A hand-maintained prelude checked by review: the failure mode
the plan named. (b) A `syn`-based parser: a new dev-dependency for a job four
`strip_prefix` calls do, since the crate's own style keeps public items at
column 0. (c) For the `capture_path` nit, setting/clearing `NOVA_SHOT_DIR` in
the test: rejected - the env is process-wide and would race the sibling tests in
the same binary, which is exactly why the original test skipped instead. Split
the pure `resolve_capture_path(shot_dir, path)` out instead and asserted all
five branches deterministically; `capture_path` keeps the env read and is still
covered end to end by `tests/reel.rs`.

Difficulty. Documenting `ready` truthfully changed the claim: the plan said "the
predicate is re-evaluated every frame until it returns `true`", but `reel_drive`
consults it every frame for the WHOLE run, so a predicate that flips back to
`false` mid-reel pauses the remaining beats. The doc says that instead of the
plan's wording.

Evidence. `cargo test -p nova_autopilot --lib --test prelude` 15+2 pass;
`cargo test --doc -p nova_autopilot` 4 compile-only doc tests (was 3);
`RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps` clean;
`cd web && npm run ci` exit 0 with `dist/wiki/dev/automation-harness/` emitted;
`cargo fmt --check` clean. The scan was proven to bite: adding a throwaway
`pub const PROBE_ONLY` to `reel.rs` failed `prelude_names_every_public_item`
with the "public but not re-exported" message, and was then reverted.

Not run locally: the display-dependent integration tests (`tests/reel.rs`,
`tests/screenshot*.rs`, `tests/autopilot_example.rs`) and workspace-wide clippy;
CI covers them. No source they touch changed except `reel.rs`'s doc text and the
private path-resolution split, both compiled by `cargo check --all-targets`.

Next time. Writing the enforcement test before the prelude body was the right
order: the "which items exist" list came out of the scan rather than out of the
plan's hand-written list, and the two agreed.

## Round 1 fixes (8cb01f55)

All eight findings fixed; none pushed back on. Two of them were the same root
cause and the more interesting one: a docs task about a freshly EXTRACTED crate
wrote the page in the tense of the crate's finished state. The opt-in snippet
demonstrated `hold(GameStates::Playing, ...)`, which is exactly the force-set
`nova_debug::harness` documents at length as forbidden for an asset-gated
`Loading -> Playing`, and the shell block gave a `NOVA_AUTOPILOT=1 cargo run
--example scenario` that is inert because `scenario` still runs the
`bevy_common_systems` copy on `BCS_AUTOPILOT`. Both are the same failure: the
page described the post-`20260802-183403` world as current fact. The fix is a
bolded framing paragraph plus per-claim tense, not a caveat footnote.

The R1.4 fix is the one that changed enforcement rather than prose: the scan
covered a hardcoded four-file list, so a FIFTH `pub mod` would have slipped the
whole DoD. `every_module_is_scanned` now derives the module set from `lib.rs`
and asserts `MODULES` matches it in both directions. Verified by sabotage
(`pub mod probe_only;` -> failure -> revert), which is what the four-file list
never was.

Re-verification after the fixes: `cargo fmt --check` clean, `cargo test -p
nova_autopilot --lib --test prelude` 15+3 pass (the new test is the +1),
`cargo test --doc -p nova_autopilot` 4 pass, `RUSTDOCFLAGS=-Dwarnings cargo doc
-p nova_autopilot --no-deps` clean, `cargo check --all-targets -p
nova_autopilot` clean, `cd web && npm run ci` exit 0 with
`dist/wiki/dev/automation-harness/index.html` emitted, and all five
`tatr proofs` proofs re-run green. The display-dependent integration tests and
workspace clippy remain CI's job.
