# Move base-mod content gen to build-time; remove the content bin's gen subcommand

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: wontdo,v0.8.0,tooling,build

## Story

As the project owner, I want the base mod's `assets/base/**/*.content.ron`
regenerated automatically when the game builds, so that the committed base
content can never silently drift from the code builders and there is no manual
"run `content gen` and commit" step to forget. This retires the `gen`
subcommand from the `content` bin entirely.

Today `nova_assets::scenario_generation::content_files()` is the single
definition of the built-in sections/scenarios; the `content` CLI's `gen`
subcommand serializes it into the committed `assets/base/**/*.content.ron` the
game loads, and `content_ron_parity.rs` asserts the two match (failing with a
"run gen and commit" message when they drift). That is a manual gate: a builder
change plus a forgotten regen leaves stale RON that only the test catches.
Moving generation into the build makes the code the source of truth and the RON
a build artifact.

This is a concrete sub-task under the tooling inventory umbrella
(20260718-152304); it executes the "fold content gen into a build-time step"
decision from that pass.

## Steps

- [ ] Move generation into the project build: add a `build.rs` step (the root
      `nova-protocol` crate, with `nova_assets` as a build-dependency, or a
      dedicated build hook) that calls `content_files()` and writes each file
      under `assets/base/`. Confirm the chicken-and-egg is fine: build-deps
      compile in a separate graph, so `nova_assets` building for the host is OK.
- [ ] Decide the artifact policy and record it: do the generated RON files stay
      committed (they must physically exist under `assets/` for the wasm/web
      static-serve path and for `trunk build`), with the build refreshing them
      in place, OR does the build emit them to a staging location the game and
      Trunk read from? Default recommendation: keep them committed, build
      rewrites in place, but justify it against reproducible-build and
      dirty-tree concerns (a build that mutates tracked files can surprise CI).
- [ ] Remove the `Gen` subcommand and `run_gen` from
      `crates/nova_assets/src/bin/content.rs`; update the bin's module docs so
      it no longer advertises `gen`.
- [ ] Repoint or retire `content_ron_parity.rs`: with the build regenerating,
      the test becomes "the committed RON matches the builders" WITHOUT naming a
      manual `content gen` command (update the `REGEN` message to name the build
      step), or it becomes a pure build-does-it check. Keep the parity guarantee
      one way or another so a stale-RON PR still fails CI.
- [ ] Verify the base game still boots and loads base content on native AND web
      (`trunk build`) after the move; a clean checkout that has never run `gen`
      must produce correct base RON purely from building.
- [ ] Sweep every reference to `content -- gen` / `content gen` and repoint it
      (LESSONS.md, web/src/wiki/dev/*, docs/design/*, README tools section
      20260718-152205, the content bin docs). Grep: `content.*gen`.

## Definition of Done

- Building the project regenerates `assets/base/**/*.content.ron` from the
  builders; a builder change picked up by a build produces matching RON with no
  manual step.
- The `content` bin no longer has a `gen` subcommand (it is now lint-only, once
  20260718-152240's lint/audit merge lands).
- Parity is still guaranteed in CI: a PR whose committed base RON disagrees with
  the builders fails, with a message that names the build step, not `content gen`.
- Native and web builds boot with correct base content from a clean checkout.

## Notes

- Single source of truth stays `nova_assets::scenario_generation`. This task
  only changes WHO invokes it (the build) and removes the hand-run CLI path.
- Sequencing: coordinate with 20260718-152240 (which merges lint+audit into a
  single `lint`); together they shrink the `content` bin to just `lint`. Land
  either order, but update the bin's docs to match whatever ships first.
- Watch the "build mutates tracked files" footgun: CI that runs the build and
  then checks `git diff --exit-code` becomes the parity gate for free, but a
  developer build that dirties the tree unexpectedly is annoying - document the
  chosen behavior in the README tools section.
- Umbrella: 20260718-152304 (tooling inventory) records this as the base-mod-gen
  build-time move; keep its catalog in sync (content bin loses `gen`).

## Decision (2026-07-20): keep `content gen`, do NOT move to build-time

Investigated during the v0.8.0 tooling flow and DECLINED the build-time move
(user call). Rationale:

- `content_files()` builds the section/scenario config types, which derive
  bevy `Reflect`, so any generator pulls in the full bevy dependency tree. A
  root `build.rs` calling it needs `nova_assets` as a BUILD-dependency, which
  compiles bevy a SECOND time in the build graph (resolver-2 keeps build-deps
  and normal deps separate) - a large first-build regression on every native
  `cargo build`/`cargo run`, plus the build would mutate tracked files
  (`assets/base/**/*.content.ron`), the dirty-tree/CI-surprise footgun the
  task Notes flagged.
- The only cheap build-time hook in the repo is `nova_meta_gen`'s Trunk
  `post_build` hook, which is web-only and pays the bevy-compile cost once per
  fresh checkout. Routing base-mod gen through Trunk was rejected: the user
  does not want Trunk building the base mod, and it would not cover native
  builds anyway.
- Net: the build-time move trades a real, repeated build-cost + tree-mutation
  footgun for removing one hand-run step that the `content_ron_parity` test
  already guards. Poor cost/benefit.

Outcome: `content gen` STAYS as a subcommand; `content_ron_parity` remains the
drift gate (it fails a stale-RON PR and names `content gen` to fix it). The
`content` bin is therefore `gen` + `lint` (not the single-`lint` end state the
umbrella 20260718-152304 sketched); that end state is not pursued. Retagged
`wontdo`, priority 0. Umbrella note updated.
