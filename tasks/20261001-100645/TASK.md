# Teach ship sections and item-loop play in wiki and Lessons

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.15.0

## User facts

- Update wiki how-to guidance for new ship sections with descriptions, diagrams, animations, and loops. Update the in-game main-menu Lessons for the same playable actions.
- This task owns ship-section operation and the item loop, not generated-world ships and regions. The latter has its own task, `20261001-103906`.
- Write against merged master and verified player flows. This task does not authorize unmerged gameplay.

## Agent findings (scouted 2026-10-01)

- On master `24f5fe343`, item-loop M0-M3 are merged (#87, #88, #91, #92, #93). `web/src/wiki/sections.md:25-68` names docking and cargo intake but its at-a-glance catalog has no Mining row. PR #98 adds installed-beam mining and finite trade; it was open at initial scouting. Recheck merge state and source before describing mining/trade as shipped.
- `docs/keeping-docs-in-sync.md` routes player behavior to `web/src/wiki/`, creator contracts to `web/src/create/`, mechanisms to `docs/`, and lesson changes to the wiki and lesson content pipeline. `web/src/docs-manifest.js` registers wiki pages and navigation. `web/src/create/sections.md`, `/create/objects.md`, `/create/lessons.md`, and `docs/sections.md` are candidate cross-checks, not an automatic rewrite list.
- `crates/nova_authoring/src/base_content/lessons.rs:1-24,45-84,127+` owns the Lessons catalog. Lessons use binding-independent action names, real wiki paths, optional focused practice scenarios, and still or 4x5/20-frame WebP loops at 10 fps. `scripts/capture-lesson-media.sh:58-99` maps each lesson to a real capture producer; `scripts/gen-lesson-media.py` supplies placeholders only. `crates/nova_authoring/tests/lesson_wiki_links.rs` checks wiki links.
- Wiki screenshots use source placeholders and `scripts/gen-web-screenshots.py` / `scripts/capture-web-shots.sh`; website loops use `scripts/capture-web-media.sh` and `web/src/assets/loops/`. Avoid borrowing frozen `news-*` media as living wiki material. Existing `build_generate`/`build_skin` lesson loops teach editing, not section operation.

## Delivery

1. Inventory actual player-visible ship sections and item-loop actions on merged master: docking, cargo intake/canisters, repair/inventory/ammo and, once merged, Mining beam/ore/trade. Read owning Rust/content and player flows. Separate player guidance from creator schema. Identify genuinely new section kinds before adding pages; do not duplicate existing docking documentation.
2. Plan a concise wiki how-to route from `web/src/wiki/getting-started.md` and `web/src/wiki/sections/`: dock -> Take/Give, align -> intake pickup/jettison, and, once M5 merges, lock -> deploy beam -> mine -> collect canister. If trading stays demo-only in New Game, state that boundary. Update `web/src/docs-manifest.js`, related links, and relevant `/create/` references only for changed contracts. Generated regional roles, wreck identity, and sector streaming belong to task `20261001-103906`.
3. Plan figures per action: labeled section/door/approach geometry, real-game close and wide frames of docking/intake, motion loops for alignment/pickup/jettison, and, once shipped, beam door/tip deployment, hit, and output. Decide which are annotated diagrams versus captured gameplay. Give each living asset a stable `wiki-*` or `loop-*` name, capture producer, source revision, and accessible caption/alt. Inspect wide and narrow layouts and real frames.
4. Add or amend focused Lessons in `crates/nova_authoring/src/base_content/lessons.rs` for verified playable verbs. Keep action IDs rebindable and wiki links resolvable; specify practice only when an existing drill teaches it, otherwise `None` or a separately approved drill. Add capture producers in `examples/` and `scripts/capture-lesson-media.sh`; update `scripts/gen-lesson-media.py` inventory, generated content/bundle references, and menu presentation as required. Placeholder art is not a finished demonstration.
5. Reconcile affected wiki, creator docs, developer docs, and CHANGELOG with current source. Remove stale or duplicate claims; do not add generated-world Lessons, unrelated gameplay, or release media.

## Verification / done when

- A reader can follow each documented shipped action in a real current build; no open-PR feature is described as live. Diagrams label real geometry and units; captured loops/frames show the stated before/after state. Inspect output on wide and narrow pages and in the Lessons pane.
- Every new lesson wiki path, action ID, media path, and practice ID validates. Run affected authoring/link/media checks, generate and lint base content, `nix develop --command mdbook build`, and web npm CI; inspect rendered docs/menu and captured lesson media. Record any absent capture evidence honestly.
- Ship-section teaching has independent source ownership, media, and acceptance evidence from the generated-world task. The task is not complete by adding prose or placeholders alone.
