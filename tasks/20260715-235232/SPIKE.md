# Spike: wiki documentation review (post-front-doors) - player / creator / developer

- DATE: 20260715-235232
- STATUS: RECOMMENDED
- TAGS: spike, docs, web

## Question

If I were a player, a creator, or a developer, would I be happy with the wiki
documentation? What works well, what could be better, and is anything stale or
useless? This is a re-review after the front-door overhaul that landed earlier
today, so the target is the *current* state, not the docs the prior spike saw.

## What changed since the last wiki review (20260715-223147)

The prior spike (`tasks/20260715-223147/SPIKE.md`) reviewed 26 pages and found
two confirmed bugs plus three "front door" gaps. Since then, one commit landed
everything it recommended:

- `4832a5b4 docs(web): wiki front doors for players, creators, and developers`
  shipped all five docs-improvement tasks plus both bug fixes.
- Both prior confirmed bugs are now CLOSED and verified fixed:
  - `modding-ron.md` no longer references the nonexistent `*.scenario.ron`
    format (rewritten to `*.content.ron` / `ContentAssetLoader`).
  - The dead `examples/03_scenario.rs` ref is now `08_scenario.rs`.
- New pages: `getting-started.md` ("Your first flight"), `glossary.md`,
  `dev/guide-author-section.md` (RON section reference).
- Nav is now 30 pages in three bands ("For players" / "For creators" / "For
  developers"), with a "Start here" category leading the player band.
- One follow-up is already OPEN and untouched: `20260715-231500` (annotate the
  HUD/radar screenshots with callout labels).

So the docs improved materially. This review confirms the overhaul worked and
finds the residue.

## Method

Three persona reviewers (player / creator / developer) each read the current
pages in their band and spot-checked claims against the code in `crates/`,
`examples/`, `webmods/`, and `assets/`. The cross-cutting facts below were then
re-verified directly.

## Headline verdict

| Audience | Verdict | One line |
|---|---|---|
| Players | Happy | Real onboarding path now exists (getting-started + glossary); accurate throughout; one unit slip and no editor page. |
| Creators | Happy | A non-programmer can now author AND launch a mod end to end (Scenarios picker is real and tested); the ship-authoring wall remains, honestly disclosed. |
| Developers | Happy | Unusually trustworthy - every example ref, symbol pointer, crate map, and schedule rule verified; one stale example count. |

The front-door overhaul succeeded. All three bands are now "happy" rather than
"mostly." What is left is small, cheap, and mostly cosmetic.

## What works well (verified, worth protecting)

- **Onboarding is real now.** `getting-started.md` gives a new player the core
  loop in a tight "first two minutes" list (New Game -> Shakedown Run; burn,
  lock, GOTO, raise weapons, fire) and links tutorial + keybinds + glossary. The
  "what do I do" gap the prior spike called the biggest is closed.
- **The no-Rust creator journey holds end to end.** author `*.content.ron` ->
  enable mod -> pick in the Scenarios picker -> Play, no Rust. The picker is real
  (`crates/nova_menu/src/lib.rs:493`), filters `hidden`, lists base+mod
  scenarios, and is pinned by tests (`picker_scenarios` includes the mod
  scenario `gauntlet_run`). This was the single most important creator claim and
  it is true and tested.
- **Accuracy is high and re-verified.** Player numbers match code exactly
  (resistance table `nova_gameplay/src/damage.rs:109-129`; verb keys
  `input/player.rs:600-630`; SOI 8x; radar cone 18deg / 20000u / 2500u). Dev:
  every `examples/NN_*.rs` ref exists, all ~14 sampled "grep for `symbol`"
  pointers resolve, the 13-row crate map matches `Cargo.toml` one-for-one, and
  the Update-vs-FixedUpdate rule is grounded in `nova_gameplay/src/plugin.rs`.
  Creator: `ScenarioConfig`, `BaseSectionConfig`, every `SectionKind` field
  matches the real structs.
- **Symbol-based anchors landed.** The ~12 line-number -> "grep for symbol"
  conversions (the drift-prone change) all resolve. This is the right long-term
  hedge against staleness.
- **Complete runnable artifacts exist.** A full assembled starter scenario in
  `guide-author-scenario` section 6; a real clone-me demo mod
  (`assets/mods/demo/`) and publish example (`webmods/gauntlet/`).
- **Concrete numbers and honest sharp-edges** carried over from before (typed
  damage table, real thresholds, and every creator page's "Sharp edges" note).

## Confirmed bugs (verified against the tree - fix first, cheap)

1. **`modding-ron.md:98` points at a directory that does not exist.** The
   "Built-ins ported" section says built-ins are "data files under
   `assets/scenarios/`". `ls assets/scenarios/` -> No such file or directory;
   the real path is `assets/base/scenarios/` (which lines 41 and 43 of the same
   file already state correctly - so line 98 is an internal contradiction). It
   also says "All four built-ins" but the base bundle ships **five** scenario
   content files (`demo`, `asteroid_field`, `asteroid_next`, `menu_ambience`,
   `shakedown_run`). Filed: `20260715-235435`.
2. **`development.md:98` undercounts the example suite.** "Ten of the twelve
   carry panic-on-failure assertions" - but `HARNESSED_EXAMPLES` in
   `tests/examples_smoke.rs:28-45` lists **18** examples (01-18). The page's
   Examples section also enumerates only 01-12 and never mentions 13-18. Stale
   from before 13-18 were added; the sentence "keep list and disk in sync" is
   itself what drifted. Filed: `20260715-235440`.

## One false alarm (recorded so it does not recur)

- **"The screenshots exist but are not wired in" is NOT a bug.** The player
  reviewer saw `<div class="figure__placeholder">Screenshot needed</div>` in the
  `.md` source and every `wiki-*.png` present in `web/src/assets/`, and concluded
  the images are unreferenced. They are not. `web/src/site.ts:30`
  (`img.onload = () => placeholder.replaceWith(img)`) swaps the placeholder for
  the real image at runtime *only once the PNG decodes*, so when the asset
  exists the player never sees "Screenshot needed" - and when it is missing the
  placeholder safely stays. This is the exact same trap the prior spike flagged;
  it will keep tricking any source-only reader (human or agent). Worth a one-line
  comment in the `.md` figure blocks, or leave this note as the standing answer.

## Remaining gaps by persona (small, mostly cosmetic)

### Players (verdict: happy)
- **Unit slip: `km` vs `u`.** The whole wiki and the glossary teach distance in
  `u` (glossary defines `u` and `u/s`), but `targeting-radar.md:43` says lock
  ranges of "roughly 20 km" and "about 2.5 km". A player has no `km`<->`u`
  mapping. Should read 20000 u / 2500 u (matches `input/targeting.rs:315,89`).
  Cheap, worth a task.
- **No player-facing editor / ship-building page.** `getting-started.md` sends
  players to Sandbox "to build and test-fly your own hull," but no page explains
  the editor (place/bind sections). The `F1` keybind and "click a section to
  bind it" are the only guidance.
- **Screenshots are unannotated raw frames.** Already tracked as OPEN task
  `20260715-231500` (HUD/radar callout labels).

### Creators (verdict: happy)
- **Ship authoring is the real wall.** Any scenario with a player ship inlines
  the whole section catalog. Honestly disclosed, and the starter file is
  deliberately ship-free - but a creator who wants their *own* ship is told to
  "copy a ship block from a shipped scenario," and there is **no copy-pasteable
  ship block in the docs**. They must open `asteroid_field.content.ron` and lift
  `player_spaceship` by hand. Biggest creator friction remaining.
- **No consolidated "which folder" map.** Base scenarios in
  `assets/base/scenarios/`, base sections in `assets/base/sections/`, mod content
  in `assets/mods/<id>/`. Correct but scattered across guides.
- **No standalone "run this file" mode / no schema validation.** Both honestly
  disclosed; they make the iterate loop heavier but are not doc defects.

### Developers (verdict: happy)
- **Stale example count** (bug 2 above) is the only wrong fact.
- **No pointer to `examples/data/`.** The examples dir has a `data/` subdir the
  docs never mention.
- **Cross-band links are unmarked.** `scenario-system.md`'s "How-to companions"
  link jumps into the "For creators" band without a note that those are the
  no-Rust authoring guides.

## Cross-cutting themes

1. **The overhaul worked - this is now a polish backlog, not a rewrite.** Every
   prior-spike recommendation shipped and verified. Ship the two bug fixes, then
   the km/u slip, then the two content gaps (player editor page, creator ship
   block) if desired.
2. **Freshness/drift is the standing risk, and it is already recurring.** Both
   new bugs are the same failure mode as last time: a hardcoded count/path in a
   historical or "keep in sync" note drifts silently as the tree grows. The
   symbol-anchor conversion helps for code refs; the remaining hazards are prose
   counts ("four built-ins", "twelve examples") and literal directory paths. A
   periodic re-check (this spike, run again) is the cheapest guard until those
   are derived rather than typed.
3. **Source-only review over-reports the screenshots every time.** The runtime
   figure-swap is invisible in the `.md`. Bake a note into the figure blocks so
   the next reviewer (and the next agent) stops re-filing it.

## Recommendation

The wiki is in good shape across all three audiences - accurate, illustrated at
runtime, well-organized, with real onboarding on every band. Do NOT rewrite.
Ship a small polish batch:

1. The two confirmed bugs (filed: `20260715-235435`, `20260715-235440`) - cheap,
   high signal, and both are freshness drift.
2. The km/u unit slip in `targeting-radar.md:43` - trivial.
3. Optional content: a player editor/ship-building page; a copy-pasteable ship
   block in the creator docs; a one-line note in figure blocks about the runtime
   swap.
4. Finish the already-open screenshot-annotation task `20260715-231500`.

## Seeded tasks

Filed now (confirmed bugs, both freshness drift):
- `20260715-235435` - fix `modding-ron.md:98` stale `assets/scenarios/` path
  (-> `assets/base/scenarios/`) and "four" -> "five" built-ins.
- `20260715-235440` - fix `development.md:98` "ten of the twelve" -> 18 harnessed
  examples, and enumerate 13-18 in the Examples section.

Recommended (for /plan when prioritized):
- Player: fix the `km` -> `u` unit slip in `targeting-radar.md`; add a
  player-facing editor / ship-building page for the Sandbox door.
- Creator: add a copy-pasteable player-ship block (or link `player_spaceship`
  from `asteroid_field.content.ron` as clone-me); add a consolidated
  "which folder holds what" content-layout map.
- Docs hygiene: add a one-line note in the `.md` figure blocks (or a wiki README)
  that `site.ts` swaps the placeholder for the real screenshot at runtime, so
  source-only reviewers stop re-filing "screenshots missing."
- Already open: `20260715-231500` - annotate the HUD/radar screenshots.
