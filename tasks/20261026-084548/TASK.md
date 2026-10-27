# Sketch v0.16.0 news post for owner review

- STATUS: CLOSED
- Later state: the approved sketch became the registered v0.16.0 page with 16 frozen figures (7 loops, 9 stills), two interactive widgets and an inspected thumbnail. The DRAFT-only statements below describe the earlier review stage, not the current page.
- PRIORITY: 85
- TAGS: v0.16.0

## User facts
- First create a v0.16.0 news-page sketch as substantial as prior feature posts, with polished prose, interactive widgets and placeholder still/loop art. User validates it before final image/loop examples and capture.
## Delivery
- Special Fable authors the draft from CHANGELOG.md and real v0.16 features in the existing news Markdown/figure conventions. Keep media news-0160-* placeholders with detailed capture briefs; preview must not imply art exists.
- Register and render only a clearly marked draft preview. Do not tag, date as released, or package fake screenshots. Follow-up media and publish live in the release task.
## Verification / Done when
- Draft has working local route/TOC/widgets/static fallback and comparable editorial depth to 0.14/0.15; user has a concrete sketch to review. Placeholder figure briefs identify needed game examples. User feedback or explicit acceptance recorded before final media.

## Sketch delivered (2026-10-26)
- At sketch approval the draft had 794 lines, 25 `##`/`###` headings (13 / 12), and 15 placeholders (6 loops, 9 stills). Its two proposed widgets (`price-factors`, `stance-ledger`) are now implemented and registered alongside `relation-matrix` and `mining-beam-checks`. The honest two-beat escort footage adds a seventh loop, so the current draft has 16 planned figures (7 loops, 9 stills). It remains a DRAFT preview, with no frozen media, tag or release commit.
- At sketch approval, `npm --prefix web run ci`, `nix develop --command mdbook build`, direct `renderMarkdownFile` (H1, 25 TOC headings, then 15 figures, four static widget hosts) and `git diff --check` passed. The post now has a local DRAFT route, four registered widgets and a provisional extra escort figure; rerun web/browser checks against the final article and actual media.
- The three `FACT CHECK` comments are resolved with owner citations: 8 s arrival grace `crates/nova_world_base/src/sector_ships.rs:88`; save on crossing `crates/nova_world_base/src/save/session.rs:378-405`, leave save `session.rs:188-196`, 240 s restore limit `crates/nova_world_base/src/save/transients.rs:56`; escort 200 m / 100 m `crates/nova_ship/src/input/ai/follow.rs:25,29`, gap added to the summed hull radii at `:116`. Scout fact checks confirmed kits, station scaling, job weights and rewards, price table, item prices, stance constants, fine, `;`, chart keys, relative times and the 10 m/s safe contact.

## Art-shot list (after owner validates the text)
| figure | kind | example / producer to build | brief |
|---|---|---|---|
| `news-0160-release-lead.webm` | loop | `news_0160_release_lead`: seed 2 military, armed-fit asserted, undock and GOTO | patrol picket undocks from an allied ring station, HELM clears, accepted Escort JOB-1 marks the hauler, the pilot locks it and starts GOTO |
| `news-0160-world-setup.webm` | loop | menu rig under Xvfb driving the wizard | name + seed chart right-drag, Next, role list stepping industrial to bounty hunter, hull card Weapons line changes |
| `news-0160-start-hulls.png` | still | `news_0160_start_hulls` dedicated 3x2 grid | six start hulls, bows left, one scale and light, intake doors visible |
| `news-0160-station-archetypes.png` | still | dedicated seed-9 producer: ring a=.25, spire a=.75, yard a=.25; living `screenshot_station_showcase` unchanged | ring four truss spokes, spire panel wings, yard struts, each framed by own hull |
| `news-0160-station-orbit.webm` | loop | `news_0160_station_orbit`: purpose-built staged fixture, NOT a seeded world cluster; generated ring (a = 0.25) on a scaled 200 m planetoid's 500 m orbit, two generated hulls for sale parked by the station, an undriven generated Enemy hull written onto a tilted 800 m orbit at loop open | ring station riding its orbit, two docked ships, turrets tracking the passing hull; caption must not imply a naturally seeded cluster |
| `news-0160-refit-catalogue.webm` | loop | docked Refit flow under the autopilot harness | Catalogue, pick PDC, ghost green on dorsal socket, R roll, fit, view eases, Placing block |
| `news-0160-station-pane.png` | still | seeded Ally start station, seed 23; docked Station pane | This station price, one genuine sale card, STBD filled, green hire notice |
| `news-0160-escort-ambush.webm` | loop | seed-2 military escort first beat | JOB-1 Escort on hauler, raiders engage, player's PDC hits; no early completion |
| `news-0160-escort-done.webm` | loop | same escort run, later beat within normal loop cap | hauler arrives, its JOB-1 mark clears, hand-in mark moves to issuer and done notice appears; no reward yet |
| `news-0160-jobs-pane.png` | still | hold procurement from first station; dock at second station | milestone, held procurement meter/inspector, three new offers with Escort Accept unavailable on unarmed hull; DONE escort at issuer cannot share this full board |
| `news-0160-market-inspector.png` | still | `market_sandbox` planet-heavy station, Iron ore selected | Price + Deal badge, Reference/Pays/Asks with three factors, MARKET strip |
| `news-0160-stances-block.png` | still | chart overview after two offences and a separate provocation | three STANCES rows with base [ALLY] badges: seeded Ally start, X OFFENCES 2/3 plus HOSTILE, Y HOSTILE countdown; no Enemy/FINE mark or Pay fine button in overview |
| `news-0160-chart-view.webm` | loop | seed 10 transport: start STN-1 on Y6, ship staged at STN-2 on Y0, no sensor-found WRK | chart Y6 then Z down through Y0; select STN-2 for SECTOR, clear for OBJECTIVES/log. 36-frame layer dwell and example-local VP9 CRF 10 preserve decoded OBJECTIVES Y6..Y0; old CRF 34 erased small digits |
| `news-0160-load-screen.png` | still | three real saved worlds in an isolated profile; selected seed 34 grants hire credits | Load rows with role icon, play time, saved-ago; chart STN-1, WRK-1/2, JOB-1 and YOU, FLEET/escort/objectives; recapture after package VERSION becomes 0.16.0 |
| `news-0160-generated-scale.png` | still | seed 10 armored hulls at 277/1,901/3,106 km, 100/265/395 m | common scale and light, bows left, intakes visible; far forward lances are armored over and NOT visually claimed |
| `news-0160-settings-presets.png` | still | Settings > Graphics top after changing Camera Shake | Custom lit and four levers; Window/VSync/Pause require scrolling, UI scale/Invert Y other tabs |

## Concept widgets to build after validation
- `price-factors` (`web/src/widgets.ts`, register in `WIDGETS`): controls = item select (ten base items), cluster type (4), civilization role (4), stock slider 0 to 3x target plus an "unkept" toggle; output = ask and bid against reference with Scarcity / Cluster / Civilization chips, Total and a `clamped` mark. Constants lifted from `crates/nova_authoring/src/base_content/market.rs:16-65` and `items.rs:49-136`; pure function `priceFactors()` exported and covered in `web/tests/widgets.test.ts`.
- `stance-ledger` (`web/src/widgets.ts`): three civilizations with seeded edges; buttons Offence, Merit, Provoke, Settle, +1 h play; badges step per `OFFENCE_STEP`/`MERIT_STEP` (3), `OFFENCE_WINDOW_SECONDS` 3600, `PROVOKED_WINDOW_SECONDS` 900, `ENEMY_DECAY_SECONDS` 7200 (`crates/nova_gameplay/src/civilization.rs:128-147`), one-hop propagation per `crates/nova_world_base/src/stance.rs`; ledger line per event; pure `stanceStep()` exported and tested.
- Both widgets should also land on `web/src/create/prices.md` and `web/src/create/stations.md#civilization-stances` so the news post is not their only host.

## Owner acceptance and draft preview (2026-10-26)
- The local `/news/0.16.0/` route and newest-first `/news/` card are marked DRAFT, use explicit placeholder figures, and were built and checked with web CI. No final media or release date was packaged.
- The owner viewed the news page and approved the sketch: "the news page looks good, go ahead and work on it." News-specific examples, widgets, captures, final registration, and release publication remain in the release task.

## Final route and media wiring (after approved captures)
- `web/webpack.config.js` `NEWS_POSTS`: prepend `{ slug: "0.16.0", version: "v0.16.0", date: <release day>, description: ... }`; the plugin list and `historyApiFallback` derive from it.
- `web/src/news.html`: add the newest-first `.post-card` with `assets/thumb-news-0.16.0.png` and an alt line.
- Figures: add each `news-0160-*` alias to `scripts/gen-web-screenshots.py` (stills) or `scripts/capture-web-media.sh` (loops); `NOVA_UNFREEZE=news-0160` while authoring.
- Then `cd web && npm run ci` and confirm `/news/0.16.0/` renders with a populated TOC sidebar.
