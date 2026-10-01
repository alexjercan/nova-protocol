# Teach new ship sections and generated-world play in wiki and Lessons

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.15.0

## User facts

- Update the wiki how-to documentation for new ship sections and world generation, with both descriptions and visual diagrams/animations/loops.
- Update the in-game main-menu Lessons for these features. Scope the writing against merged master and the relevant PRs, not only planned mechanics.
- This is a high-priority documentation/media task, not authorization to implement unmerged gameplay.

## Agent findings (scouted 2026-10-01)

- On master `24f5fe343`, item-loop M0-M3 are merged (#87, #88, #91, #92, #93). `web/src/wiki/sections.md:25-68` names docking and cargo intake but its at-a-glance catalog has no Mining row. `web/src/wiki/getting-started.md:14-30` describes New Game with derelicts; revise claims only when the relevant feature merges.
- PR #98 (M5) adds installed-beam mining, finite trade and related UI; PR #96 adds the regional generator prototype; PR #99 adds streamed generated ships/wrecks. At scouting time all are OPEN, so claims about M5 mining/trade and regional streaming are conditional on merge. Do not present provisional balance numbers, persistence, encounters, reputation, AI or weathered wreck art as shipped features. Confirm current PR heads and master again before writing.
- `docs/keeping-docs-in-sync.md` routes player behavior to `web/src/wiki/`, creator contracts to `web/src/create/`, mechanisms to `docs/`, and lesson changes to the wiki plus the lesson content pipeline. `web/src/docs-manifest.js` registers wiki pages and navigation. `web/src/create/sections.md`, `/create/objects.md`, `/create/lessons.md`, `docs/sections.md`, `docs/architecture.md`, and `docs/ship-layout-sense.md` are candidate cross-checks, not an automatic rewrite list.
- `crates/nova_authoring/src/base_content/lessons.rs:1-24,45-84,127+` owns the Lessons catalog. Lessons use binding-independent action names, real wiki paths, optional focused practice scenarios, and still or 4x5/20-frame WebP loops at 10 fps. `scripts/capture-lesson-media.sh:58-99` maps each lesson to a real capture producer; `scripts/gen-lesson-media.py` supplies placeholders only. `crates/nova_authoring/tests/lesson_wiki_links.rs` checks wiki links.
- Wiki screenshots use source placeholders and `scripts/gen-web-screenshots.py` / `scripts/capture-web-shots.sh`; website loops use `scripts/capture-web-media.sh` and `web/src/assets/loops/`. Avoid borrowing frozen `news-*` media as living wiki material. The existing `build_generate`/`build_skin` lesson loops (`scripts/capture-lesson-media.sh:94`) are editor lessons, not evidence that live regional generation is already taught.

## Delivery

1. Inventory player-visible sections and world behavior on the target merged revision: docking, cargo intake/canisters, repair/inventory/ammo, and, once merged, Mining beam/ore/trade and generated-role ships/wrecks. Check the owning Rust/content paths and actual player flows. Separate player guidance (how to use/recognize) from creator schema and internal generator details. Explicitly identify any genuinely new section kinds before adding pages; do not duplicate existing docking documentation.
2. Plan a concise wiki how-to route from `web/src/wiki/getting-started.md` and the `web/src/wiki/sections/` pages. Give the player a repeatable dock -> Take/Give, align -> intake pickup/jettison, and (once M5 merges) lock -> deploy beam -> mine -> pick up sequence; distinguish demo-only trade from New Game if that boundary remains. For streamed ships (once #99 merges), explain roles, region/provenance, intact versus lootable wreck, and sector-revisit limitations without implying combat relations. Update `web/src/docs-manifest.js`, related links and relevant `/create/` references only for actual changed contracts.
3. Provide a figure/loop plan per how-to step: labeled section/door/approach geometry diagram, real-game close and wide frames of dock and intake, 2-second motion loops for alignment/pickup/jettison, and, once shipped, door/tip deployment, mining hit/output, role/skin and wreck comparison. Choose which are annotated diagrams and which must be captured from rendered gameplay. Give each living asset a stable `wiki-*` or `loop-*` name, producer, source revision and accessible caption/alt. Inspect both full-width and narrow layout; never claim a render is proof until its frame is inspected.
4. Add or amend focused Lessons in `crates/nova_authoring/src/base_content/lessons.rs` for verified playable verbs. Keep action IDs rebindable and wiki links resolvable; specify practice only when a focused existing drill genuinely teaches the behavior, otherwise `None` or a separately approved drill. Add capture producers in `examples/` and `scripts/capture-lesson-media.sh` for real lesson footage; update `scripts/gen-lesson-media.py` inventory, generated content/bundle references, and the menu presentation as required by the actual lesson format. Do not call placeholder art a finished demonstration.
5. Reconcile affected `web/src/wiki/`, creator docs, developer docs and CHANGELOG against current source. Remove stale or duplicate claims, including the New Game trader/role/persistence boundary when appropriate. Do not add unrelated gameplay or alter release media.

## Verification / done when

- A reader can follow each documented shipped action in a real current build; no unmerged feature is described as live. Wiki diagrams label real geometry and units, and captured loops/frames show the stated before/after state. Inspect output on wide and narrow pages and in the in-game Lessons pane.
- Every new lesson's `wiki_path`, action ID, media path, and practice ID is validated. Run affected authoring/link/media checks, generate and lint base content, `nix develop --command mdbook build`, `cd web && npm run ci`, and inspect rendered docs/menu pages and captured lesson media. Record any absent renderer/capture evidence honestly.
- Clear source ownership for new media and explicit PR dependencies (#98, #96, #99); the task is not complete by adding prose or placeholders alone.
