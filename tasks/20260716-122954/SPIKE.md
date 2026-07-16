# Spike: what should v0.7.0 contain - content & scenarios, bugfixes & performance, UI/UX polish

- DATE: 20260716-122954
- STATUS: RECOMMENDED
- TAGS: spike, planning, v0.7.0

## Question

v0.6.0 ("modding & authoring") shipped 2026-07-16. The user set three goals for
v0.7.0: (1) more content and interesting scenarios/assets, (2) bug fixes and
performance, (3) small UI/UX improvements in the game. Which backlog tasks
serve those goals, what new tasks do the goals imply that the backlog does not
contain, and in what order? A good answer is a concrete task list with
priorities encoding execution order, plus an explicit out-of-scope line, all
captured in `docs/plans/20260716-v0.7.0-plan.md`.

## Context

State at the start of this spike (master at 48f0060d, tag v0.6.0):

- **Backlog**: 25 OPEN tasks, none IN_PROGRESS, zero open `bug` tasks. One
  v0.6.0 leftover (20260708-224303 SFX integration test). Everything else is
  `backlog` at p0.
- **Content is the thin spot.** Base game ships 5 scenarios, of which only
  shakedown_run and asteroid_field are real gameplay (demo is a modding
  showcase, asteroid_next a transition, menu_ambience a backdrop); one portal
  mod (Gauntlet Run). Assets: 1 asteroid texture, 3 same-style skyboxes, 1
  hull model + turret/torpedo part models, 17 sounds (audio is well stocked),
  no fonts shipped. 7 section prototypes, 4 spawnable object kinds, 19
  scenario actions - the authoring vocabulary from v0.6.0 is rich, the content
  written in it is not. There is still NO explicit win/lose state in the game.
- **v0.6.0 unblocked the big content tasks.** The RON format, content model,
  bundle loader, editor baseline, AI state machine + engagement flight, and
  nova_ui all CLOSED, so the capital-combat vertical slice (20260708-203659),
  AI retreat (20260709-225734), ship prototypes (20260714-134115), settings
  menu (20260711-180511) and in-game web fonts (20260714-214329) are all
  unblocked.
- **No known bugs; perf is measured only at the modding layer.** Code carries
  2 TODOs (torpedo explosion visuals; asset-loader refactor). The scenario
  dispatch hot path is benchmarked and fine at realistic rates
  (tasks/20260714-083331/modding-perf-report.md). There is no frame-time
  baseline for actual gameplay scenes (dense asteroids + particles + combat),
  native or web - so "improve performance" currently has no numbers to aim at.
- **UI/UX gaps found in survey**: Settings panel is an empty stub with a Back
  button; no ammo/magazine readout on the HUD even though multi-type magazines
  and reload shipped (20260712-133349); health bar is a generic screen-space
  bar (diegetic HP task 20260711-202901); Bevy default font everywhere while
  the web app has a real type identity (Rajdhani/Inter/JetBrains Mono).
- **Web/docs are healthy** (wiki complete, CI blocking and green). Only media
  gap: 4 missing news-card thumbnails, now named `thumb-news-0.3.0..0.6.0.png`
  after the news merge while `scripts/gen-web-screenshots.py` still lists the
  old `thumb-devlog-3/4/5.png` names; blocked on user content choices
  (task 20260715-092658), not on capture tech.
- An old v0.5.0-era roadmap spike (tasks/20260708-203517/SPIKE.md) penciled
  v0.7.0 in as "modding scripting, weapon variety, editor save/load, docs
  pass". The user's stated goals supersede that sketch: content, fixes/perf,
  UI polish.

## Options considered

- **A. Content-first release (recommended).** Theme "content & polish": the
  flagship capital-combat slice finally gives the game a win/lose frame, a
  scenario pack + asset variety fill out the world, and the perf/UI strands are
  small, bounded riders. Pros: exactly the user's stated goals; dogfoods the
  v0.6.0 authoring stack (RON scenarios, portal) with real content, which is
  the best way to find its bugs; the engineering was explicitly built for this
  ("shippable game" identity, hard-sci-fi capital combat). Cons: art/asset
  production is a new kind of work for this repo; win/lose needs design.
- **B. Authoring-completion release.** Pull in the editor scenario builder
  "the rest" (20260714-081703) and ship-prototype kind, making v0.7.0 a second
  modding release. Rejected: does not match the user's goals; hand-written RON
  is proven sufficient for authoring content (Gauntlet Run); the builder is a
  tool, not content, and building it before more content exists inverts the
  dogfooding order.
- **C. Platform/input release.** Gamepad menu navigation + mobile virtual pad
  (20260714-001140), keybind icons + remapping (20260710-231927). Rejected for
  the theme: big-ticket input work, explicitly parked as "later stage polish"
  by the user; only the settings-menu surface (a UI/UX item) is pulled in.
- **D. Do nothing / maintenance-only.** Always a candidate the day after a
  release. Rejected: the content gap is the game's biggest weakness and the
  user has named it.

## Recommendation

Option A: theme **"content & polish - make it a game you can win or lose, with
more places to fly."** Three strands, sized like v0.6.0 (~15 tasks):

1. **Content** (headline): capital-combat vertical slice owning the explicit
   win/lose frame (20260708-203659, p85); AI retreat so fights end
   (20260709-225734, p65); a two-scenario content pack, one published as a
   second portal mod (new task, p60); an asset variety pack - skyboxes,
   asteroid/planet texture variants, a hull model variant (new task, p50);
   real per-scenario picker thumbnails (20260715-220011, p28); ship-prototype
   content kind stays conditional (20260714-134115, p24) - activate only when
   the slice/pack actually shares a ship definition, respecting the recorded
   deferral decision.
2. **Bugfixes & performance**: a gameplay frame-time baseline on heavy scenes,
   native + web, then fix only what the numbers justify (new task, p40) -
   same measure-first gate v0.6.0 used for dispatch; the low-end spawn-less
   visual mode rides those numbers and lands as the settings menu's graphics
   preset (20260525-133013, p22); SFX integration test carried over from
   v0.6.0 (20260708-224303, p20). Policy, not a task: bugs found while
   building/playtesting content are filed as `bug` tasks at p90+ and fixed
   within the release - that is how "bug fixes" happens in a repo with zero
   known bugs.
3. **UI/UX polish** (small, bounded): settings menu content - audio sliders,
   graphics quality preset, read-only keybind reference; full remapping stays
   backlog (20260711-180511, p45); in-game web fonts (20260714-214329, p35);
   ammo/magazine HUD readout (new task, p32); diegetic HP spike
   (20260711-202901, p30); lock acquire/lock cue polish (20260708-165703,
   p25).

Runners-up that stay backlog: editor builder "the rest" (081703) is the
natural v0.8.0 headline once this release has proven the content demand;
gamepad/mobile (001140) and keybind icons/remap (231927) wait for the
settings surface to exist; piccolo scripting (162010) remains gated on the
declarative format hitting a ceiling, which more content will test.

## Open questions

- **Art sourcing.** Skyboxes/textures/models: procedural generation, sourced
  CC0 packs, or hand-made in Blender (an `art/` dir with sources exists)?
  Decide in the asset-pack task's own spike step; license gate (cargo-about
  is code-only) needs an equivalent answer for art attribution.
- **Win/lose shape.** Fail state = ship destruction, objective timeout, or
  both? Victory = objective chain only? The slice task's /plan pass decides;
  the scenario vocabulary (OnDestroyed, variables, NextScenario) looks
  sufficient, and any gap found is itself v0.7.0 modding-surface work.
- **Ship-prototype consumer.** Does the slice/pack actually want a shared
  enemy ship definition? If yes, 20260714-134115 activates mid-release; if
  every scenario inlines its ships, it stays parked.
- **News thumbnails** (20260715-092658) stay blocked on the user picking
  source images; also reconcile the thumb-devlog-N vs thumb-news-X.Y.Z rename
  when picked up.

## Next steps

Direction-level tasks (priorities encode order; `/plan` breaks each into steps
when picked up). Release plan: `docs/plans/20260716-v0.7.0-plan.md`.

- tatr 20260708-203659 (p85): capital-combat vertical slice + win/lose frame (base game: New Game progression + picker entry)
- tatr 20260709-225734 (p65): AI retreat on low integrity
- tatr 20260716-123535 (p60): story campaign mod - alt storyline across multiple scenarios, on the portal (amended from "scenario content pack")
- tatr 20260716-124722 (p55): Gauntlet Run 2.0 - a real parkour-style gauntlet (added in review)
- tatr 20260716-123544 (p50): asset variety pack - skyboxes, textures, hull variant + mod-shippable resources
- tatr 20260711-180511 (p45): settings menu content (main menu + pause menu)
- tatr 20260716-123551 (p40): gameplay performance baseline + measured fixes
- tatr 20260714-214329 (p35): in-game web fonts
- tatr 20260716-123556 (p32): ammo/magazine HUD readout
- tatr 20260711-202901 (p30): diegetic HP spike
- tatr 20260715-220011 (p28): real per-scenario picker thumbnails
- tatr 20260708-165703 (p25): lock acquire/lock cue polish
- tatr 20260714-134115 (p24, conditional): ship-prototype content kind
- tatr 20260525-133013 (p22): low-end graphics preset (rides settings + perf numbers)
- tatr 20260708-224303 (p20): SFX event->sound integration test (v0.6.0 carryover)

## Amendments (20260716, user review)

The user reviewed the seeded scope and adjusted it; the plan doc and tasks
carry the detail:

- The scenario content pack (20260716-123535) became a STORY CAMPAIGN mod: an
  alt storyline across multiple chained scenarios with a real narrative,
  downloadable from the portal - the base game keeps its own storyline.
- The vertical slice (20260708-203659) ships in the BASE game, both as the
  next New Game progression scenario and as a standalone Scenarios-picker
  entry.
- New task 20260716-124722 (p55): Gauntlet Run 2.0 - more gates, obstacles
  and hazards, a real parkour-map-style gauntlet, re-published as a portal
  version bump.
- The asset variety pack (20260716-123544) also owns the mod-resources gap:
  bundles must be able to ship their own PNGs/models/audio and link them with
  mod-relative references (today mods can only point at base-game asset
  paths).
- Settings menu (20260711-180511): also reachable from the pause menu.

## Fix record

(Appended by each implementing task as it lands - what shipped, the headline
number, a pointer to its TASK.md.)
- 20260716-125856 (outcome frame): SHIPPED - Outcome action + overlay with
  Continue/Retry/Main Menu, Enter parity; shakedown death dogfoods it.
  Landed 9a27efac after 2 review rounds. tasks/20260716-125856/.
- 20260708-203659 (vertical slice): SHIPPED - Broadside, a three-act
  chapter two (neutral hauler, corvette ambush, torpedo-gunship climax)
  in the picker + chained from shakedown's Victory screen; new authorable
  ship allegiance; loader skybox install made deferred (mod-sky crash
  fixed); example 19 walks defeat->retry->victory in CI.
  tasks/20260708-203659/.
