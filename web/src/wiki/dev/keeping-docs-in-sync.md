# Keeping docs in sync

Nova Protocol documents itself across several surfaces, each aimed at a
different reader. None of them updates itself, so a code change is not finished
until the docs it invalidates are fixed in the **same task**. This page is the
map: what the surfaces are, what to touch when you change code, and what to do
when you cut a release. It is the overview; the detailed, command-level steps
live in [Building & running](../development/) under "Versioning and release" and
"Writing the release news post".

## The documentation surfaces

| Surface | Where | Audience | Kept in sync when |
| --- | --- | --- | --- |
| `CHANGELOG.md` | repo root | everyone (terse, complete) | any user-visible change |
| News | `web/src/news/*.md` -> `/news/` | players + modders | each feature release |
| Player wiki | `web/src/wiki/*.md` -> `/wiki/` | players | player-facing behavior changes |
| Dev wiki | `web/src/wiki/dev/*.md` -> `/wiki/dev/` | contributors | internals / formats change |
| Tutorial | `web/src/tutorial.html` -> `/tutorial/` | new players | the first-flight flow changes |
| `docs/` | repo root | agents/contributors | transient records only (see `docs/README.md`) |

`CHANGELOG.md` is the exhaustive machine reference (every version, terse,
grouped by subsystem). News is the story (one rich post per feature release).
The wiki is the manual. They overlap on purpose - the cost of that overlap is
that one code change can carry several doc obligations, which is what the map
below makes explicit.

## When you change code

Before you commit, ask three questions and act on each "yes":

1. **Did user-visible behavior change?** Add a line to `CHANGELOG.md` under
   `## [Unreleased]`, in the right subsystem section (Gameplay & Flight, Combat
   & Weapons, Ships & Sections, Scenarios & Objectives, Modding & Mod Portal,
   Interface & HUD, Web & Platform, Audio & Visuals, Performance, Fixes,
   Internals & Tooling). One terse line; tag a format break **(breaking)**.
2. **Did player-facing behavior change** (controls, HUD, a verb, a section, a
   weapon, a scenario primitive, the modding UX)? Update the player wiki
   page(s) it affects, and the tutorial if the first-flight flow moved. A wiki
   page drifting behind the game is the exact failure to avoid.
3. **Did internals, architecture, or a data format change?** Update the dev
   wiki page(s) that describe them. A RON / bundle / catalog format break
   especially must land in `dev/modding-ron.md` and/or `dev/mod-portal.md` in
   the same task, or every mod author reads a lie.

### The dependency map

Which docs to check when you touch a given area. "Check" means read it and fix
it if the change made it wrong - not every change touches every listed page.

| Code area (crate / dir) | Player wiki | Dev wiki | Also |
| --- | --- | --- | --- |
| Ship sections, integrity, typed damage, ammo (`nova_gameplay/sections`, `integrity`) | `sections.md` (+ section children), `hud.md` | `dev/sections.md`, `dev/guide-author-section.md`, `dev/guide-add-section.md` | CHANGELOG |
| Flight, controller, camera (`nova_gameplay/input`, `camera`) | `flight-autopilot.md`, `keybinds.md` | `dev/architecture.md` | CHANGELOG |
| Targeting, radar, weapons, turrets, torpedoes (`nova_gameplay` targeting/hud) | `targeting-radar.md`, `combat-weapons.md`, `hud.md` | `dev/architecture.md` | CHANGELOG |
| Gravity wells, factions, world (`nova_gameplay`) | `gravity-wells.md`, `factions.md` | | CHANGELOG |
| Scenario engine: events, filters, actions, variables, objects (`nova_scenario`, `nova_events`) | `scenarios.md` | `dev/scenario-system.md`, `dev/guide-author-scenario.md`, `dev/guide-extend-scenarios.md` | CHANGELOG |
| Modding data format, bundles, catalog, local cache (`nova_mod_format`, `nova_modding`) | `modding.md` | `dev/modding-ron.md`, `dev/guide-make-a-mod.md` | CHANGELOG **(breaking?)** |
| Mod portal + generator (`nova_portal_gen`, `nova_modding`) | `modding.md` | `dev/mod-portal.md`, `dev/modding-ron.md` | CHANGELOG |
| Menus, editor, UI (`nova_menu`, `nova_editor`, `nova_ui`) | `hud.md`, `sections.md` | `dev/guide-add-section.md` | tutorial, CHANGELOG |
| App assembly, plugin order, states (`nova_core`, `nova_assets`) | | `dev/architecture.md`, `dev/project-tour.md` | CHANGELOG |
| Content CLI: gen/lint/audit subcommands (`nova_assets` bin `content`) | | `dev/guide-author-scenario.md`, `dev/guide-make-a-mod.md`, `dev/modding-ron.md` | CHANGELOG |
| The website itself (`web/`) | | `dev/development.md`, [this page](../keeping-docs-in-sync/) | |

When a change lives in `bevy-common-systems` (the pinned git dependency), fix it
there first, bump the `rev` in `crates/nova_gameplay/Cargo.toml`, and note the
bump in `CHANGELOG.md`.

## When you cut a release

The full command-level checklist is in
[Building & running -> Versioning and release](../development/). At the doc
level, a release means:

1. **`CHANGELOG.md`**: promote `## [Unreleased]` to `## [<version>] -
   <YYYY-MM-DD>`, leave a fresh empty `[Unreleased]`, merge any duplicate
   subsystem headings that grew during the cycle, and update the compare links
   at the bottom.
2. **News**: a **feature release** (`0.X.0`) gets a new post at
   `web/src/news/<version>.md`, registered in `NEWS_POSTS` in
   `web/webpack.config.js` with a card added to `web/src/news.html`. A **patch
   release** (`0.X.Y`) gets **no post of its own** - fold it into its parent
   feature post's `## Point releases` section instead. Full steps: [Writing the
   release news post](../development/).
3. **Wiki**: sync any player or dev pages the release's changes touched (use the
   map above). Do this as you go during the cycle, not in a scramble at release.
4. **Build check**: `cd web && npm run ci` (format check, lint, build) must be
   green; confirm `/news/` and the new post render, and the section TOC sidebar
   is populated.

## Adding or renaming a page

Adding a wiki page means editing `web/webpack.config.js` (the `WIKI_DOC_PAGES`
list) **and** the manifest `web/src/wiki-pages.ts` (which drives the sidebar,
search and see-also). Adding a news post edits `NEWS_POSTS` in
`web/webpack.config.js` plus a card in `web/src/news.html`. Retiring a URL adds
a stub to `REDIRECTS` in `web/webpack.config.js`. Verify any of these with
`cd web && npm run ci`.
