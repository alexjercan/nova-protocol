# Spike: wiki documentation review (player / creator / developer)

- DATE: 20260715-223147
- STATUS: RECOMMENDED
- TAGS: spike, docs, web

## Question

The wiki was rebuilt this session (markdown pipeline, developer guides, the
three audience bands, blog conversion, drawer-scroll). If I were a player, a
creator, or a developer, would I be happy with this documentation? What works
well, and what could be better?

## Method

Reviewed all 26 wiki pages across the three bands, plus what landed recently
(this session added: the markdown wiki pipeline 195621; the intent-based IA +
5 dev guides 204358; the player-page + blog markdown conversions 205825/210609;
the filter-docs + load-test rework 212600; the audience bands 215924; drawer
scroll 222307). Three persona reviewers each read the real pages and spot-checked
claims against the code; the cross-cutting facts below were verified directly.

## Headline verdict

| Audience | Verdict | One line |
|---|---|---|
| Players | Mostly happy | Well-written, accurate, actually illustrated - but no "start here" front door. |
| Creators | Mostly | A determined RON author can ship a mod, but hits avoidable walls; one page is stale-wrong. |
| Developers | Mostly yes | Unusually accurate, excellent extension guides; a dead ref + missing conventions/contrib workflow. |

## What works well (cross-cutting, and worth protecting)

- **The pages are actually illustrated.** All 16 player-wiki figures have real
  in-engine screenshots captured in `web/src/assets/` (0 missing); the
  "Screenshot needed" text is only the source-level fallback markup that
  `site.ts` swaps at runtime. The "wiki has no images" worry is a non-issue.
- **Accuracy is high and fresh.** The dev reviewer spot-checked ~15 `file:line`
  anchors and nearly all were dead-on; the crate map matches the workspace
  `Cargo.toml` exactly; Bevy 0.19 / avian3d check out.
- **Concrete numbers - rare for a game wiki.** combat-weapons.md ships the full
  per-section resistance table (EMP 3.0x into controllers, 0.1x into hull);
  targeting/flight give real thresholds (0.25s lock hold, ~18deg cone, 20km ship
  lock, `v = sqrt(mu/r)`). This turns "damage types matter" into strategy.
- **The best teaching content is genuinely good:** `guide-author-scenario.md`
  (events->filters->actions in order, the per-event `id`/`other_id` table, the
  worked objective loop), and the two extension guides (`guide-add-section`,
  `guide-extend-scenarios`) as first-PR on-ramps ("the compiler is your
  checklist").
- **Honest about sharp edges** (no scenario picker, asset-path verbosity, ships
  inlining their section catalog, 5s pair-event recurrence) - creators are not
  blindsided.
- **Navigable:** the audience bands, cross-linking / see-also, and the new
  drawer-scroll persistence make it pleasant to move around.

## Confirmed bugs (verified against the tree - fix first, cheap)

1. **`modding-ron.md` is stale on the core file format.** It says scenarios are
   `*.scenario.ron` files under `assets/scenarios/` (lines 7, 13, 36, 38, 93,
   292). There are ZERO `*.scenario.ron` files in the repo; every scenario is a
   `Scenario((...))` item inside a `*.content.ron`. A creator who follows this
   page authors a file the loader never picks up. The creators front door
   (`modding.md`) advertises this page as "the RON data format reference".
2. **`scenario-system.md:26` cites `examples/03_scenario.rs`, which does not
   exist** (it consolidated into `08_scenario.rs`; `03` is now
   `03_hull_section.rs`). The one broken pointer a newcomer would hit.

## Gaps by persona (from the three reviews)

### Players (verdict: mostly)
- **No "Getting started / your first flight" page** - the biggest gap. Every
  page assumes you already launched and know Shakedown Run is the intro.
- The **tutorial is only reachable via inline links**, not a first-class wiki
  entry, even though it is the real onboarding.
- **No glossary; the `u` / `u/s` distance-speed unit is never defined.**
- **Control-theory jargon leaks** in a few spots (flight "PD attitude loop /
  per-tick allocation / nulling net torque"; controller "PD controller").
- **Screenshots are unannotated** - the HUD shot makes you map ~8 named widgets
  onto pixels by hand.
- **`scenarios.md` / `factions.md` lean developer-facing**; the closest thing to
  an "objectives / what am I doing" page reads like design docs. No tactics/tips
  page tying the resistance table to real fights.

### Creators (verdict: mostly)
- **The stale `modding-ron.md`** (bug 1) is the one that wastes an afternoon.
- **`modding-ron.md` reads as a changelog, not a reference** (serde rationale,
  `AssetRef` bounds) - a no-Rust creator gets none of the authoring vocabulary
  the front door promised.
- **No creator-facing section-authoring reference.** The flagship demo mod ships
  a `Section((... Hull(...)))`, but the `Section` / `BaseSectionConfig` /
  `SectionKind` RON vocabulary lives only in dev internals + the Rust guide.
- **No complete, copy-pasteable starter scenario** - only fragments; the real
  `asteroid_field.content.ron` isn't linked as "clone this".
- **No pure-RON way to launch your own scenario:** section 7 tells creators to
  edit `NEW_GAME_SCENARIO_ID` (Rust) or chain via `NextScenario`. Honestly
  disclosed, but still a wall for the target audience.
- **Journey mismatch:** make-a-mod says "copy the demo mod", whose headline is a
  section overlay the scenario guide never taught.

### Developers (verdict: mostly yes)
- **Dead `03_scenario.rs` ref** (bug 2).
- **No hard "start here" reading order / dev landing** - it lives only inside
  `project-tour.md` prose and the manifest `related` arrays.
- **The two-clocks (Update vs FixedUpdate) rule is shown but never explained** -
  the diagram stops at "here's the split"; a contributor still doesn't know
  which schedule their system belongs in (the classic Bevy footgun).
- **Coding conventions are implicit; no external-contributor workflow**
  (branch / `cargo check && cargo fmt` / driving example / PR / CI).
- **`nova_debug` tooling is named but not shown** (inspector, wireframe,
  `--debugdump`).
- **Hardcoded `~line N` anchors will drift** (all correct today, but line 1667
  of a 1700-line file is fragile) - prefer symbol/grep targets.

## Cross-cutting themes

1. **Each band needs a front door.** Player "getting started", creator "one
   complete clone-me artifact", developer "numbered reading order". The content
   exists; it just isn't assembled into an on-ramp.
2. **Reference vs guide confusion.** `modding-ron.md` is a changelog sold as a
   reference; `scenarios.md` mixes player-facing "what's in the world" with
   authoring vocabulary.
3. **Freshness/drift is the standing risk.** "No X yet" claims (data-driven
   scenarios TODO, no scenario picker) and hardcoded line numbers go stale
   silently. Symbol-based anchors + a periodic re-check would help.
4. **The dev/creator boundary is slightly wrong.** Section authoring is *data*
   (RON) but is stranded in dev internals; and there is no no-Rust scenario
   launch, which pushes creators into the dev world they were told to avoid.

## Recommendation

The docs are in good shape - accurate, illustrated, well-organized. Ship a small
"docs polish" batch rather than a rewrite: fix the two bugs first (cheap, high
impact), then add the three front doors, then the freshness/drift items. Seeded
tasks below; the two bugs are filed now.

## Seeded tasks

Filed now (confirmed bugs):
- 20260715-223551 - fix `modding-ron.md` stale `*.scenario.ron` format + retarget
  it as a real reference (or drop the "reference" framing on `modding.md`).
- 20260715-223555 - fix the dead `examples/03_scenario.rs` ref in
  `scenario-system.md` (-> `08_scenario.rs`).

Recommended (for /plan when prioritized):
- Player front door: a "Getting started / your first flight" page + promote the
  tutorial into the nav + a small glossary defining `u`/`u/s` + annotate the HUD
  and radar screenshots + trim the control-theory jargon.
- Creator completeness: a full assembled starter scenario (and link
  `asteroid_field.content.ron` as clone-me) + a creator-facing "author a section
  (RON)" reference + a front-and-centre honest note on launching your own
  scenario.
- Developer onboarding: a "start here" reading-order callout / dev landing +
  an "Update vs FixedUpdate - which schedule?" rule in architecture.md + a
  "Contributing a change" workflow + a debug-tooling section; move the extend
  guides to symbol-based anchors.
