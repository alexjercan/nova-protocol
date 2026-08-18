# Keeping docs in sync

Nova Protocol documents itself across several surfaces, each aimed at a
different reader. None of them updates itself, so a code change is not finished
until the docs it invalidates are fixed in the **same task**. This page is the
map: what the surfaces are, what to touch when you change code, and what to do
when you cut a release. It is the overview; the detailed, command-level steps
live in [Building and running](development.md) under "Versioning and release"
and "Writing the release news post".

## The documentation surfaces

| Surface | Where | Audience | Kept in sync when |
| --- | --- | --- | --- |
| `CHANGELOG.md` | repo root | everyone (terse, complete) | any user-visible change |
| News | `web/src/news/*.md` -> `/news/` | players + modders | each feature release |
| Player wiki | `web/src/wiki/*.md` -> `/wiki/` | players | player-facing behavior changes |
| Creator docs | `web/src/` -> `/create/` | mod authors | a data format or the modding UX changes |
| Dev book (this book) | `docs/` -> `/dev/` | contributors | internals, architecture, or the dev workflow change |
| rustdoc | local: `cargo doc --open` | contributors | with the code (doc comments travel in the same diff) |
| Tutorial | `web/src/tutorial.html` -> `/tutorial/` | new players | the first-flight flow changes |

`CHANGELOG.md` is the exhaustive machine reference (every version, terse,
grouped by subsystem). News is the story (one rich post per feature release).
The player wiki is the manual; `/create/` is the modding manual and the
exhaustive construct catalog. This book is the developer's map - how to run,
how to extend, where things live - and rustdoc is the API detail underneath
it. They overlap on purpose - the cost of that overlap is that one code change
can carry several doc obligations, which is what the map below makes explicit.

**`docs/` is the source of this book, not a scratchpad.** The old
ephemeral-docs model (free-form working notes under `docs/`, compiled and
wiped at each release) is retired. Everything under `docs/` is a maintained
book chapter; transient working files live outside the repo, and task-scoped
records live in `tasks/<id>/`.

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
3. **Did internals, architecture, or a data format change?** Update the book
   chapter(s) that describe them. A RON, bundle, catalog, or portal format
   change must also land in the matching `/create/` reference or publishing
   page in the same task, or every mod author reads a lie.

### The dependency map

Which docs to check when you touch a given area. "Check" means read it and fix
it if the change made it wrong - not every change touches every listed page.
Player wiki names are `web/src/wiki/` pages; `/create/...` names are the
creator pages on the site; linked names are chapters of this book.

| Code area (crate / dir) | Player wiki | Dev book / creator docs | Also |
| --- | --- | --- | --- |
| **The crate layout itself**: a split, merge, rename, or move (`crates/*`) | | [Architecture](architecture.md) (crate map + dependency graph + assembly), [Project tour](project-tour.md) (crate map + change-X table), and THIS page's own row keys | CHANGELOG (Internals) |
| Ship sections, integrity, typed damage, ammo (`nova_ship/sections`, `nova_gameplay/integrity`) | `sections.md` (+ section children), `hud.md` | [Ship sections internals](sections.md), [Add a ship section](guide-add-section.md), `/create/sections/` | CHANGELOG |
| The derived skin, its plate vocabulary and skin styles (`nova_ship/sections/shell_*`, `skin_*`, `scripts/gen-greebles.py`) | `sections.md` | [Ship sections internals](sections.md) (the derived skin), `/create/styles/`, `/create/base-content/` (style ids, greeble assets), `/create/objects/` (the ship's `skin` / `style` fields) | CHANGELOG |
| Flight, controller, camera (`nova_ship/input`, `camera`) | `flight-autopilot.md`, `keybinds.md` | [Architecture](architecture.md) | CHANGELOG |
| Targeting, radar, weapons, turrets, torpedoes (`nova_ship` targeting/sections, `nova_hud`) | `targeting-radar.md`, `combat-weapons.md`, `hud.md` | [Architecture](architecture.md) | CHANGELOG |
| Gravity wells, factions, world (`nova_gameplay`) | `gravity-wells.md`, `factions.md` | | CHANGELOG |
| Scenario engine: events, filters, actions, variables, objects (`nova_scenario`, `nova_events`) | `scenarios.md` | [Scenario engine](scenario-system.md), `/create/author-a-scenario/`, [Extend the scenario engine](guide-extend-scenarios.md), **the creator reference** (`/create/scenarios/`, `/create/events/`, `/create/filters/`, `/create/actions/`, `/create/objects/`, `/create/expressions/` - a new/changed construct MUST land there, it is the exhaustive catalog) | CHANGELOG |
| Modding data format, bundles, catalog, local cache (`nova_mod_format`, `nova_modding`) | `modding.md` | `/create/mod-files/`, `/create/base-content/` (overlay rules, dep://base) | CHANGELOG **(breaking?)** |
| The ship content kind: what a hull IS vs what a spawn is (`nova_scenario/objects/ship.rs`, `nova_authoring/base_content/ships`) | `sections.md` | `/create/ships/`, `/create/objects/` (the Spaceship spawn), `/create/mod-files/`, `/create/reference/` | CHANGELOG **(breaking?)** |
| Mod portal + generator (`scripts/gen-portal.py`, `nova_modding`) | `modding.md` | `/create/publish-a-mod/` | CHANGELOG |
| Menus, editor, UI (`nova_menu`, `nova_editor`, `nova_ui`) | `hud.md`, `sections.md` | [Add a ship section](guide-add-section.md) | tutorial, CHANGELOG; **theme tokens: `web/design/nova_ui_rework_poc.html` is the source for BOTH `nova_ui/src/theme.rs` and `web/src/style.css`; the site draws the PHOSPHOR skin only** |
| Automation drivers, the env contract, the completion protocol, the probe harness (`nova_autopilot`, `nova_probe`, `nova_probe_cli`) | | [Automation harness](automation-harness.md), [Building and running](development.md) (probe sections) | CHANGELOG **(env rename? breaking for every run script)** |
| App assembly, plugin order, states, the game binary's own flags (`nova_core`, `nova_assets`, `src/main.rs`) | | [Architecture](architecture.md), [Project tour](project-tour.md), [Building and running](development.md) ("Launching a scenario from the command line") | CHANGELOG |
| Content CLI: gen/lint subcommands, the base content builders (`nova_authoring`, the game binary's `content` subcommand) | | `/create/author-a-scenario/`, `/create/sections/`, [Add a ship section](guide-add-section.md), `/create/publish-a-mod/`, `/create/mod-files/`, [Scenario engine](scenario-system.md), [Ship sections internals](sections.md), `/create/base-content/` (the id/asset catalog - a builder change that adds, renames or rebalances an id lands there) | CHANGELOG |
| The website itself (`web/`) | | [Building and running](development.md), this page | |
| Local dev servers (`scripts/serve-web.sh`, `scripts/serve-mods.sh`, `scripts/preview-web.sh`, `web/webpack.config.js`, `Trunk.toml`) | | [Building and running](development.md) ("Local web preview"), `/create/publish-a-mod/` ("Preview the repository portal") | README.md quick start + scripts table |

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
[Building and running -> Versioning and release](development.md). At the doc
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
   release news post](development.md). Post conventions the 0.7.0 post sets:
   structure the body as `##` sections with `###` subsections - the build
   derives the sticky TOC sidebar from those headings, so a flat post gets an
   empty TOC; and use the figure-placeholder format (a `.figure` block that
   auto-upgrades to its screenshot once `scripts/gen-web-screenshots.py`
   packages the image) rather than inlining an `<img>`.
3. **Wiki and creator docs**: sync any player or creator pages the release's
   changes touched (use the map above). Do this as you go during the cycle,
   not in a scramble at release.
4. **This book**: same rule - chapters stay current through the cycle, and
   `nix develop --command mdbook build` must be green (a broken link is a
   build warning; treat warnings as failures). There is no release-time wipe
   step: `docs/` carries no transient content to wipe.
5. **Build check**: `cd web && npm run ci` (format check, lint, test, build)
   must be green; confirm `/news/` and the new post render, and the section
   TOC sidebar is populated.

## Adding or renaming a page

- **A chapter of this book**: create `docs/<page>.md` and list it in
  `docs/SUMMARY.md`. `mdbook build` warns on a SUMMARY entry whose file is
  missing - treat that as a failure.
- **A site wiki page**: edit `web/webpack.config.js` (the page list) **and**
  the manifest `web/src/wiki-pages.ts` (which drives the sidebar, search and
  see-also).
- **A news post**: edit `NEWS_POSTS` in `web/webpack.config.js` plus a card in
  `web/src/news.html`.
- **Retiring a URL**: add a stub to `REDIRECTS` in `web/webpack.config.js`.

Verify any of the `web/` changes with `cd web && npm run ci`.
