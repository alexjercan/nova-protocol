# Lore encyclopedia

The world and story reference for Nova Protocol, rendered by the website at
`/lore/`. These Markdown files are the source; there is no separate lore book
build. `index.md` is the short setting introduction and categorized directory.

## Reference structure

- [Encyclopedia](index.md) introduces the time, place, and central setting
  tension without company histories, character introductions, or plot summaries.
  Its directory links to every published article, grouped by category.
- Background articles hold the detailed world reference and its open decisions:
  [Economy](economy.md), [Society and settlement](society.md),
  [Law and power](law-and-power.md), and [Ships and travel](ships-and-travel.md).
- `places/` contains location articles, beginning with [Keystone](places/keystone.md).
- `organizations/` contains named company and institution articles, including
  [EarthWorks Industrial](organizations/earthworks-industrial.md),
  [Farspan Logistics](organizations/farspan-logistics.md), and the locally founded
  [Clearwell Waterworks](organizations/clearwell-waterworks.md).
- `characters/` contains the five approved crew members:
  [Jonah Mercer](characters/jonah-mercer.md),
  [Leila Haddad](characters/leila-haddad.md), [Tomas Vega](characters/tomas-vega.md),
  [Rina Okafor](characters/rina-okafor.md), and [Samir Bell](characters/samir-bell.md).
  These articles establish their starting identities, not their unreleased outcomes.
- `seasons/` contains shared production outlines, starting with the
  [Season 1 water-station arc](seasons/season-1.md). It records the approved
  direction, character arcs, working proposals, and open decisions. This outline
  is not registered for public website publication yet.

Add individual companies, places, and institutions as their identities are
approved. Create new subject directories and navigation sections only when they
have articles. Do not add empty directory groups, placeholder character pages,
or a second overview or contents page. Put general world rules in background
articles and named histories in the relevant subject articles. Do not supply
missing names and specifications for completeness. Do not recover earlier lore
from Git.

These articles are an editorial reference to the agreed world, not an official
account published by an in-world authority.

## Narrative and shared canon

The main narrative belongs in the [web comic](../comics/) and mainline campaign,
not in a second prose novel. Readers should understand the comic without first
reading this reference. Article links provide optional depth. Short anecdotes
or document excerpts can illustrate an article without becoming the main story
delivery format.

The encyclopedia, comic, and mainline campaign share one canon. The comic and
campaign follow the same major events, character decisions, and outcomes.
Missions can add work between those events, not a contradictory history.
Record approved world facts in the relevant reference. Season and episode
outlines record the full shared story plan without retelling it as a novel.
Keep proposed alternatives separate from approved facts. Comic episodes do not
need to correspond one-to-one with missions or repeat every gameplay action.

The player acts through the whole ship, with an authored captain and restrained
game dialogue. Longer comic conversations and interiors can become comms and
cinematic exterior shots in the campaign. Do not infer an on-foot game from a
scene inside a ship or station.

## Lore and wiki ownership

Lore describes the world, its people, and the consequences of its physical and
social rules. The wiki describes the game and what a player can do. Keep lore
prose focused on life in the setting, not controls, HUD elements, balance values,
or explanations of simulation shortcuts.

Each maintained detail has one authoritative home. Do not copy wiki passages,
widgets, or numerical specifications into lore, or import gameplay constants as
world laws. A brief shared principle can provide context in both without making
two versions of a technical reference. Stable world figures are authored choices,
not automatic consequences of the current implementation.

Use separate related-reading links where the other surface answers a useful,
different question. Do not force every lore article to have a gameplay equivalent.
Keep implementation gaps and contradictions with the affected story outline or
tracked task, not as invented physical explanations in the world articles. An
unpiloted ship's simulation exemption, for example, does not make empty vessels
immune to gravity in the setting.

Check current capabilities against code, the [player wiki](../wiki/), the
[creator reference](../create/), and the [developer docs](../../../docs/).
Lore does not maintain a parallel table of gameplay statistics or engine defaults.

## Website integration

Register public pages in `LORE_PAGES` and the `lore` section of
[`../docs-manifest.js`](../docs-manifest.js). That manifest owns publication,
URLs, sidebar navigation, search, and related articles. Keep the categorized
links in `index.md` aligned with that public page list. `README.md` is an authoring
guide and is not registered or published. Register an outline only when it is
intended to be public, with its draft and spoiler status stated explicitly.

Article links are relative to their published directory URL, not their source
file. For example, `/lore/places/keystone/` links to `../../economy/` and
`../../../assets/lore/keystone-exterior-concept.svg`. Use trailing-slash page
URLs, including before an anchor. This works at both `/` and the deployed
`/nova-protocol/` prefix.

The shared website supplies navigation and typography. Encyclopedia infoboxes
and illustration plates are scoped to lore articles in [`../lore.css`](../lore.css).
From `web/`, run `node tests/lore.test.js` to check directory coverage, search
headings, article links, and assets at root and project prefixes. Build with
`npm run build`, then inspect `/lore/` and nested articles at desktop and mobile
widths.

## Illustrations

Illustrations are SVG files in `../assets/lore/`.
The unassigned station concept appears in [Society and settlement](society.md).
Named place articles carry separate exterior studies, and company infoboxes
carry emblem and wordmark proposals. Caption relationship diagrams as references
and artwork as visual concepts. Concepts do not establish a location, design,
scale, construction history, or official branding without separate approval.
The five character portraits are likewise appearance studies, not fixed ages,
ancestries, clothing, or likenesses. They have no military rank insignia.

The crew portraits share a drawing template with individually authored face,
hair, clothing, and palette geometry in `../../../scripts/gen-lore-portraits.py`.
Run `python3 scripts/gen-lore-portraits.py` from the repository root to render them,
or add `--check` to verify the saved SVGs without writing. Keep the generator and
its outputs together. Other current images are directly authored SVGs.

Use images for the subject: portrait studies for people, exterior studies for
places, identity concepts for companies, and diagrams for physical or supply
relationships. The landing illustration is a compact setting view, not an orbital
map. Do not add screenshots or interface diagrams to explain fictional technology.

Use direct SVG for diagrams, emblems, and simpler illustrations. Use a script
when repeated geometry or regeneration provides a clear benefit; do not build
a generator for every image. For a generated image, commit both its generator
and output, and change the generator rather than hand-editing the output.

### Color and presentation

Use [the unassigned station illustration](../assets/lore/industrial-station-concept.svg)
as the encyclopedia's palette reference: green-black backgrounds, sage and
olive surfaces, pale green highlights, and restrained amber accents. This
applies to both place illustrations and company identity proposals.

The comic's art direction is restrained full color, with green as a recurring
accent tied to the CRT visual language. It is not tinted green throughout.
Approved designs remain consistent between formats; the encyclopedia treatment
does not define physical paint, company livery, skin tones, or planetary colors.

The exterior, branding, and portrait SVGs retain their underlying color values. An
embedded `encyclopedia-green` filter on the inner `artwork` group supplies the
encyclopedia's presentation, including when the image is opened directly. Keep the
filter off the root SVG: Firefox does not apply it there. A full-color export
can omit the artwork group's `filter` attribute without changing the drawing
or its base colors. Those colors remain working art choices, not approved world
specifications. The unassigned station concept and reference diagrams already
use the green palette and do not need this treatment.
