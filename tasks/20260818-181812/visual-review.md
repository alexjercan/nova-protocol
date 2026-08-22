# Web visual review - 2026-08-22

This review covers the landing page, current player wiki, creator documentation,
v0.11.0 post, shipped web media, and deterministic capture machinery. Historical
news posts are immutable. Only v0.11.0 may receive release-post improvements.

The approved direction is landing-first and site-wide. Landing/news media may be
staged but must show real behavior. Player documentation shows ordinary controls
and outcomes. Creator documentation uses controlled diagnostic scenes. A visual
may be reused only when it proves the same claim.

## Inventory

| Surface | Current figure slots | Widgets | Folded prose | Missing referenced media |
| --- | ---: | ---: | ---: | ---: |
| Landing | 6 | 0 | 0 | 0 |
| Player wiki | 33 figures / 63 named slots | 16 | 16 | 38 |
| Creator docs | 0 figures / 1 named slot | 0 | 39 | 1 |
| v0.11.0 news | 12 | 8 | 0 | 11 |

The asset tree contains 36 PNG files, five of which are generated section icons,
plus two WebM loops. All current game captures were packaged before the latest
v0.11.0 presentation and graphics work. The active landing/wiki/create/v0.11.0
surfaces reference 32 existing assets: the banner, five icons, 24 screenshots,
and two loops.

The screenshot coverage report sees 103 image references and 67 gaps, but 28 of
those gaps belong to historical news and must stay untouched. The active wiki's
38 gaps are:

- 25 section-catalog thumbnails;
- five section behavior loops;
- NOVA OS open, map, ship, and terminal media;
- GOTO arrival and radar-lock dwell loops;
- sandbox-range and scenarios-picker stills.

The creator docs have one incomplete scenario-picker shot. They otherwise do
not declare visuals, even on the long sections, ships, and styles references.

## Main findings

### 1. The first viewport does not show the game

The landing hero spends its full viewport on a 3:2 logo card, tagline, and
buttons. The brand art is good and should remain available, but the page asks a
new player to scroll before seeing one ship. The strongest current behavior - a
ship taking visible damage and coming apart - belongs above the fold.

Recommendation: make a close gameplay loop the hero background or main frame,
with the Nova Protocol mark and calls to action over a protected dark region.
Use the logo art as a reduced fallback/poster or brand lockup rather than as the
whole visual proposition.

### 2. The landing page tells the pre-v0.11.0 story

Its six stills are accurate to their original captures but visually repeat the
same bare, pale ship in dark asteroid fields. Several are wide enough that the
claimed action is a small HUD glyph. The editor shot predates the visual gallery
story, the combat and juice shots do not show the new damage grammar, and the
static autopilot image cannot prove a flip-and-burn.

A simple recapture would refresh UI chrome but preserve the weak story. The
landing features should become five current promises:

1. Build a ship you can see - parts gallery, socket placement, live skin.
2. Fly the real hull - GOTO flip, braking plume, and final settle.
3. Fight through geometry - readable rounds, torpedoes, and point defence.
4. Ships wear damage and come apart - cracks, section failure, wreckage.
5. Read the cockpit - contextual HUD and NOVA OS as two views of ship state.

Gravity remains important, but it is evidence inside flight rather than one of
five equal front-door stories. Its dedicated wiki page remains the deep link.

### 3. Existing current documentation captures are stale as a set

The contact sheet has a consistent old look: unclad semantic hulls, the prior
menu, repeated asteroid framing, and several subjects too small to read at web
width. Re-running the existing examples will update UI, but many examples
explicitly reconstruct inline hulls and therefore discard whole-hull skin/style
configuration. Each producer needs a claim review, not a blind recapture.

Keep without recapture:

- the five generated 44 px section icons;
- the brand banner as a fallback/brand asset;
- historical `news-090-*` and `parts-viewer-*` assets in their original posts.

Keep provisionally:

- `wiki-section-*` closeups, until the catalog turntable replaces them with one
  current, consistent capture family;
- `wiki-gravity.png`, because it still proves orbit geometry clearly.

Recapture or replace:

- all six `feature-*` landing stills;
- tutorial menu, radar, orbit, and combat shots;
- wiki combat, HUD, flight, radar, settings, sections, and ordnance shots;
- both current loops after their replacement families exist.

`torpedo-blast.webm` especially needs replacement. Its detonation fills the frame
with flat orange before the useful aftermath. `spine-cut.webm` proves the graph
cut, but the rocks crowd the subject and the hull does not represent the new
skin/damage presentation.

### 4. Player prose should become visual-first, not merely shorter

The player wiki is strong and well sourced, but some pages explain a visual
mechanism in several visible paragraphs before or after showing it. Existing
`details.explain` behavior already supports the requested spoiler pattern,
including hash links that open a folded target.

Use this page contract:

1. Keep the player decision or result visible in one or two short paragraphs.
2. Show the image, loop, or widget.
3. Keep one caption visible that says what to notice.
4. Put mechanism, edge cases, and worked explanation in a closed
   `<details class="explain">` labelled `How it works` or `Show the details`.

Never fold required tutorial steps, controls, compatibility warnings, or the
minimum fact needed to act. A visual is an alternative explanation, not an
excuse to hide instructions.

Best targets:

- `combat-weapons.md`: fold cover edge cases, barrel-discipline mechanics,
  point-defence assignment details, and penetration derivations behind their
  new visuals; keep tactical advice visible.
- `flight-autopilot.md`: it already follows this pattern; add the GOTO loop and
  tighten the visible handling result.
- `sections.md`: lead damage/collapse with motion, then fold the exact damage
  grammar. Fix visual/prose drift while doing the pass.
- section child pages: show each part acting before the stats catalog.
- `hud.md`: use an annotated current frame plus a contextual-state comparison;
  fold the complete per-widget inventory.
- `nova-os.md`: already uses folds well. Its missing media is the problem, not
  its structure.
- `getting-started.md`: keep all numbered actions visible. Replace stale shots
  and add only the sandbox/editor bridge; do not turn the tutorial into hidden
  prose.

No new visual is needed for keybinds, glossary, or factions. Their tables,
definitions, and existing relation widget are the right forms.

### 5. Creator documentation has the largest explanatory gap

The creator section is almost entirely prose, tables, code, and Mermaid. That is
correct for exhaustive references, but four authored contracts are spatial and
currently ask readers to imagine the result:

- `styles.md`: plate surface classes, seven reliefs, fixture seats, alignment,
  priority, density, and the four shipped styles;
- `sections.md`: collider versus art, link-point frames, socket mating, turret
  joint trees, and exhaust frames;
- `ships.md`: section graph, collapse threshold, derived skin, and per-spawn
  modifications;
- `author-a-scenario.md`: the final in-game picker and playable result.

Recommended creator visuals:

1. A style atlas: the same hull in all four styles plus labelled representative
   fixtures. Reuse the v0.11.0 greeble atlas.
2. A relief/seat diagnostic plate sheet generated from the real skin derivation.
3. A socket-mating loop with normals, chosen sockets, roll, and a refused
   intersection. Reuse the editor family, but retain debug overlays here.
4. A section-frame plate: collider, render mesh, link points, local axes, and
   exhaust/muzzle direction on one controlled part.
5. A ship anatomy comparison: authored section graph, derived cladding, and
   final styled hull.
6. A scenario result pair: the RON flow diagram beside the scenario picker and
   the live objective/victory outcome.

Do not decorate every event, filter, or action entry. Mermaid and code are
better for exhaustive vocabulary. Add one event-filter-action lifecycle diagram
to the scenario tutorial/reference and keep the individual entries textual.

### 6. v0.11.0 is the only news surface in scope

The post already has the correct narrative and 12 precise slots. It needs a
reuse pass, not a rewrite. Historical posts and their unresolved placeholders
remain historical.

The v0.11.0 media should establish shared capture families rather than twelve
unrelated examples. Its lead, editor, ordnance, styles, scenarios, and damage
assets can also serve the landing page or current docs when the claim matches.
The collider comparison and round-type comparison remain release-specific
because they explain this version's change rather than a standing player task.

## Capture-family map

One deterministic example may emit several related assets. This is preferable
to one example per filename because the set, lighting, actors, and assertions
stay shared.

| Family | Primary outputs | Reuse |
| --- | --- | --- |
| Destruction | hero/release lead, damage stages, clean sever/collapse tail | landing, v0.11.0, sections, combat |
| Editor build | gallery choice, socket mate, live skin, finished build | landing, v0.11.0, getting started, creator sections |
| Flight | GOTO flip/brake/settle, orbit still | landing, flight wiki, first flight |
| Ordnance | Lance/Serpent pair, per-mount point defence, clear aftermath | landing, v0.11.0, combat, torpedo bay |
| Cockpit | contextual HUD states, lock dwell, current combat lock | landing, HUD, radar, first flight |
| Styles | derived-skin build, four-style turntable, fixture atlas, relief sheet | v0.11.0, creator styles/ships |
| Section catalog | five kind closeups and 25 isolated catalog thumbnails | section overview and child pages |
| NOVA OS | power-on loop, terminal, map, ship app | landing cockpit story, NOVA OS wiki |
| Scenario surfaces | menu carousel, picker, sandbox range, example-mod picker | v0.11.0, scenarios, first flight, creator tutorial |
| Diagnostics | round travel and collider before/after | v0.11.0 and creator mechanism docs only |

The first implementation batch should build Destruction, Editor build, Flight,
Ordnance, and Cockpit. Those five families replace the landing page and cover
the highest-traffic player pages. Styles and Scenario surfaces follow. Section
catalog then fills 30 wiki gaps in one producer family. NOVA OS can use its
existing screenshot producers as a base. Diagnostics are last because they do
not improve the front door.

## Pipeline findings

The capture foundation is sound:

- examples use `AppBuilder`, seeded sets, scripted production input, named
  assertions, fixed framing, and explicit capture gates;
- stills are 1920x1080;
- loops are 1280x720 at 30 fps and currently only 273-320 KB, well under the
  3 MB limit;
- placeholders retain useful static descriptions until media exists.

The pipeline needs expansion before shipping many loops:

1. `gen-web-screenshots.py` knows the old 31-figure manifest but none of the
   v0.11.0, active missing wiki, creator, or catalog outputs.
2. `capture-web-media.sh` hard-codes only two loop producers.
3. The screenshot report classifies `news-0110-*` as historical because its
   prefix rules only know `news-090-*` as current.
4. Catalog thumbnails are unclassified even though one turntable producer can
   own all 25.
5. `site.ts` starts every image request immediately and sets every loop to
   `preload="auto"`. Adding several landing loops under this behavior would
   load the full page at once. Intersection-based loading must land with the
   first moving landing media.
6. Reduced motion stops autoplay and exposes controls, but there is no explicit
   poster/first-frame asset. Capture families should define a good first frame;
   a poster is needed only where frame zero is blank or transitional.

## Proposed landing behavior

Recommended default:

- one eager hero loop with a dark, legible first frame and static fallback;
- four feature loops/stills loaded only near the viewport;
- motion paused while off-screen;
- no audio;
- reduced motion shows a representative still and an explicit play control;
- captions and alt/ARIA text state the action, not the filename;
- the old logo image remains the fallback/brand mark.

The landing should use five feature rows rather than preserving six obsolete
slots. Existing filenames may break; this is preferable to aliases that keep an
old information architecture alive.

## Decisions before implementation

### Landing structure

- Preserve six current feature headings and only replace media: smaller prose
  change, but the page still underplays destruction, skins, and the editor.
- Replace them with the five current promises above: one fewer row, stronger
  v0.11.0 identity, and better reuse with player docs. Recommended.

### Hero treatment

- Keep the logo card as the hero and place motion immediately below it: safest
  text contrast and smallest CSS change, but the first viewport still does not
  prove gameplay.
- Put the destruction loop in the hero with the logo/CTA over a protected
  region: stronger front door and recommended, but requires eager-versus-lazy,
  fallback, and mobile crop work.

### Creator scope

- Add only style and socket visuals in this release: fastest creator improvement,
  but leaves ships and scenario onboarding text-only.
- Implement all six creator visuals as three shared families (Styles, Editor
  build, Scenario surfaces): more example work, but no one-off decorative shots
  and a coherent authored-contract story. Recommended after the landing/player
  batch.

### Catalog scope

- Continue showing placeholder thumbnails until each part has a hand-framed
  capture: highest individual quality, but 25 persistent holes and duplicated
  setup.
- Build one deterministic turntable that emits all 25 thumbnails plus the five
  kind closeups: consistent framing and full coverage. Recommended.
