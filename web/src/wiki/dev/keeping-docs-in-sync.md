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
   wiki page(s) that describe them. A RON, bundle, catalog, or portal format
   change must also land in the matching `modding/` reference or publishing
   page in the same task, or every mod author reads a lie.

### The dependency map

Which docs to check when you touch a given area. "Check" means read it and fix
it if the change made it wrong - not every change touches every listed page.

| Code area (crate / dir) | Player wiki | Dev wiki | Also |
| --- | --- | --- | --- |
| **The crate layout itself**: a split, merge, rename, or move (`crates/*`) | | `dev/architecture.md` (crate map + dependency graph + assembly), `dev/project-tour.md` (crate map + change-X table), and THIS page's own row keys | CHANGELOG (Internals) |
| Ship sections, integrity, typed damage, ammo (`nova_ship/sections`, `nova_gameplay/integrity`) | `sections.md` (+ section children), `hud.md` | `dev/sections.md`, `dev/guide-add-section.md`, `modding/sections.md` | CHANGELOG |
| The derived skin, its plate vocabulary and skin styles (`nova_ship/sections/shell_*`, `skin_*`, `scripts/gen-greebles.py`) | `sections.md` | `dev/sections.md` (the derived skin), `modding/styles.md`, `modding/base-content.md` (style ids, greeble assets), `modding/objects.md` (the ship's `skin` / `style` fields) | CHANGELOG |
| Flight, controller, camera (`nova_ship/input`, `camera`) | `flight-autopilot.md`, `keybinds.md` | `dev/architecture.md` | CHANGELOG |
| Targeting, radar, weapons, turrets, torpedoes (`nova_ship` targeting/sections, `nova_hud`) | `targeting-radar.md`, `combat-weapons.md`, `hud.md` | `dev/architecture.md` | CHANGELOG |
| Gravity wells, factions, world (`nova_gameplay`) | `gravity-wells.md`, `factions.md` | | CHANGELOG |
| Scenario engine: events, filters, actions, variables, objects (`nova_scenario`, `nova_events`) | `scenarios.md` | `dev/scenario-system.md`, `modding/author-a-scenario.md`, `dev/guide-extend-scenarios.md`, **the modding reference** (`modding/scenarios.md`, `modding/events.md`, `modding/filters.md`, `modding/actions.md`, `modding/objects.md`, `modding/expressions.md` - a new/changed construct MUST land there, it is the exhaustive catalog) | CHANGELOG |
| Modding data format, bundles, catalog, local cache (`nova_mod_format`, `nova_modding`) | `modding.md` | `modding/mod-files.md`, `modding/base-content.md` (overlay rules, dep://base) | CHANGELOG **(breaking?)** |
| The ship content kind: what a hull IS vs what a spawn is (`nova_scenario/objects/ship.rs`, `nova_authoring/base_content/ships`) | `sections.md` | `modding/ships.md`, `modding/objects.md` (the Spaceship spawn), `modding/mod-files.md`, `modding/reference.md` | CHANGELOG **(breaking?)** |
| Mod portal + generator (`scripts/gen-portal.py`, `nova_modding`) | `modding.md` | `modding/publish-a-mod.md` | CHANGELOG |
| Menus, editor, UI (`nova_menu`, `nova_editor`, `nova_ui`) | `hud.md`, `sections.md` | `dev/guide-add-section.md` | tutorial, CHANGELOG; **theme tokens: `web/design/nova_ui_rework_poc.html` is the source for BOTH `nova_ui/src/theme.rs` and `web/src/style.css`; the site draws the PHOSPHOR skin only** |
| Automation drivers, the env contract, the completion protocol, the probe harness (`nova_autopilot`, `nova_probe`, `nova_probe_cli`) | | `dev/automation-harness.md`, `dev/development.md` (probe sections) | CHANGELOG **(env rename? breaking for every run script)** |
| App assembly, plugin order, states, the game binary's own flags (`nova_core`, `nova_assets`, `src/main.rs`) | | `dev/architecture.md`, `dev/project-tour.md`, `dev/development.md` ("Launching a scenario from the command line") | CHANGELOG |
| Content CLI: gen/lint subcommands, the base content builders (`nova_authoring`, the game binary's `content` subcommand) | | `modding/author-a-scenario.md`, `modding/sections.md`, `dev/guide-add-section.md`, `modding/publish-a-mod.md`, `modding/mod-files.md`, `dev/scenario-system.md`, `dev/sections.md`, `modding/base-content.md` (the id/asset catalog - a builder change that adds, renames or rebalances an id lands there) | CHANGELOG |
| The website itself (`web/`) | | `dev/development.md`, [this page](../keeping-docs-in-sync/) | |
| Local dev servers (`scripts/serve-web.sh`, `scripts/serve-mods.sh`, `scripts/preview-web.sh`, `web/webpack.config.js`, `Trunk.toml`) | | `dev/development.md` ("Local web preview"), `modding/publish-a-mod.md` ("Preview the repository portal") | README.md quick start + scripts table |

### "Check" means re-derive, not grep

The 20260806-121625 refactor is the cautionary tale. Its lanes DID sweep docs -
at the name level ("does the page name the new crate"), and the pages did. What
survived was every claim BETWEEN the names: a dependency graph still drawing
`nova_ui` as menu-and-editor-only, a crate map missing four crates, commands
attributed to the crate they moved out of. The after-benchmark's `docs` persona
then failed a control question by trusting one of those stale claims
(task 20260809-213446). So when a row above says "check", it means: re-derive
every crate name, module path, command, and dependency direction the page
asserts against the current tree. A page that names the new thing can still
describe the old one.

Two structural traps this map cannot catch by itself:

- The map's own row keys are crate/dir names, so a structural refactor
  invalidates the MAP too - that is what the first row is for.
- A lane-per-change epic has no lane whose job is the cross-cutting sweep.
  Give the sweep its own step (or lane) whenever `crates/*` changes shape.

## When you cut a release

The full command-level checklist is in
[Building & running -> Versioning and release](../development/). At the doc
level, a release means:

1. **`docs/` compile-and-wipe** (ephemeral-docs model): `docs/` is free scratch
   during the cycle; before the tag, distil durable reference into the wiki,
   then clear everything under `docs/` except its `README.md`. (Plans live in
   tatr tasks, not `docs/plans`.)
2. **`CHANGELOG.md`**: promote `## [Unreleased]` to `## [<version>] -
   <YYYY-MM-DD>`, leave a fresh empty `[Unreleased]`, merge any duplicate
   subsystem headings that grew during the cycle, and update the compare links
   at the bottom.
3. **News**: a **feature release** (`0.X.0`) gets a new post at
   `web/src/news/<version>.md`, registered in `NEWS_POSTS` in
   `web/webpack.config.js` with a card added to `web/src/news.html`. A **patch
   release** (`0.X.Y`) gets **no post of its own** - fold it into its parent
   feature post's `## Point releases` section instead. Full steps: [Writing the
   release news post](../development/). Post conventions the 0.7.0 post sets:
   structure the body as `##` sections with `###` subsections - the build
   derives the sticky TOC sidebar from those headings, so a flat post gets an
   empty TOC; and use the figure-placeholder format (a `.figure` block that
   auto-upgrades to its screenshot once `scripts/gen-web-screenshots.py`
   packages the image) rather than inlining an `<img>`.
4. **Wiki**: sync any player or dev pages the release's changes touched (use the
   map above). Do this as you go during the cycle, not in a scramble at release.
5. **Build check**: `cd web && npm run ci` (format check, lint, test, build) must be
   green; confirm `/news/` and the new post render, and the section TOC sidebar
   is populated.

## Adding or renaming a page

Adding a wiki page means editing `web/webpack.config.js` (the `WIKI_DOC_PAGES`
list) **and** the manifest `web/src/wiki-pages.ts` (which drives the sidebar,
search and see-also). Adding a news post edits `NEWS_POSTS` in
`web/webpack.config.js` plus a card in `web/src/news.html`. Retiring a URL adds
a stub to `REDIRECTS` in `web/webpack.config.js`. Verify any of these with
`cd web && npm run ci`.
