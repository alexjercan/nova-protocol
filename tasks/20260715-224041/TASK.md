# Developer docs: start-here reading order, Update-vs-FixedUpdate rule, contributing workflow, debug tooling

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: docs,web,feature

From the docs review spike 20260715-223147 (developer persona). The dev wiki is
accurate and the extension guides are excellent, but a newcomer lacks a hard
reading order, the two-clocks footgun is diagrammed-but-not-explained, and there
is no contribution workflow or debug-tooling walkthrough.

## Goal

Complete the contributor on-ramp: a start-here path, the Update-vs-FixedUpdate
rule, a "Contributing a change" workflow, and a debug-tooling section; reduce
line-anchor drift.

## Steps

- [x] Add a "Start here" reading-order callout to the top of
      `web/src/wiki/dev/project-tour.md` (the For-developers front door): a
      numbered path Project tour -> Architecture -> Building & running -> pick a
      guide. verify-first: read `renderSidebar`/`renderIndex` and decide whether
      to also repeat a small callout atop each dev page or add a dedicated dev
      landing; a callout on the tour is the minimum.
- [x] Add an "Update vs FixedUpdate - which schedule does my system go in?" rule
      to the Frame flow section of `web/src/wiki/dev/architecture.md`: which work
      belongs in FixedUpdate vs Update, why gameplay is duplicated, and what
      breaks if a system lands in only one. verify-first: derive it from the
      two-clocks record - `docs/LESSONS.md` (`two-clocks`, `global-transform-
      stale-in-fixedupdate`) and `tasks/20260711-103527/SPIKE.md` - do NOT invent
      the rule.
- [x] Add a "Contributing a change" subsection to
      `web/src/wiki/dev/development.md`: branch, `cargo check && cargo fmt`,
      add/extend the driving example, open a PR, CI runs `cargo test --workspace`.
      Mirror the repo's real workflow (`AGENTS.md`, the existing development.md
      release/task-tracking sections); do not contradict them.
- [x] Add a "Debug tooling" subsection to `development.md`: the `nova_debug`
      plugin under `--features dev` (inspector, wireframe, overlays), and the
      debug CLI flags `--norender` and `--debugdump` (schedule graph). Ground in
      `src/main.rs` (flag parsing) and the `debug`/`dev` feature in the root
      `Cargo.toml`.
- [x] Reduce anchor drift in `web/src/wiki/dev/guide-add-section.md` and
      `guide-extend-scenarios.md`: replace hardcoded `~line N` citations with
      symbol-based pointers (name the type/fn + "grep for it") where practical -
      the guides already name every symbol, so the line numbers add fragility
      without much value (spike flagged e.g. `ScenarioObjectKind` "~line 1667").
- [x] Verify: `npm run ci` green; serve + headless-eyeball the edited pages;
      confirm the Update/FixedUpdate rule matches the two-clocks spike.

## Notes

- Grounding: two-clocks in `docs/LESSONS.md` + `tasks/20260711-103527/SPIKE.md`;
  debug flags in `src/main.rs`; contribution conventions in `AGENTS.md` and
  `web/src/wiki/dev/development.md`.
- All five sub-items are edits to existing dev pages (+ one callout); land as one
  cohesive dev-docs polish. Keep the house style and the mermaid diagrams intact.
