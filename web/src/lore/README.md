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
- `ships/` contains named vessels: Kaveri, Ebro, Gantry, Bastion, Foundation,
  Altair, and the pirate pair Redress and Windfall. Articles hold their
  established identities, not unreleased events or invented specifications.
- `characters/` contains Kaveri's five crew members:
  [Jonah Mercer](characters/jonah-mercer.md),
  [Leila Haddad](characters/leila-haddad.md), [Tomas Vega](characters/tomas-vega.md),
  [Rina Okafor](characters/rina-okafor.md), and [Samir Bell](characters/samir-bell.md).
  Gantry's crew is [Nadia Sen](characters/nadia-sen.md),
  [Owen Park](characters/owen-park.md), and [Ivo Marin](characters/ivo-marin.md).
  [Elena Ward](characters/elena-ward.md) is Clearwell's co-founder and Baikal's
  resident manager. These articles establish starting identities, not unreleased
  injuries or outcomes.
- `seasons/` contains shared production outlines, starting with the
  [Season 1 water-station arc](seasons/season-1.md). It records the approved
  direction, character arcs, working proposals, and open decisions. These are
  production sources, not public encyclopedia articles.
- Keep unpublished comic page studies, generators, renders, and review evidence
  with the active story task, outside the public archive and website copy rules.
  TODO(20260908-161328): Develop the shared story and opening comic drafts.
  Draft designs, dialogue, and staging are not additions to the released story.

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

## Production and publication

Keep season and scene outlines as shared production references. Adapt their
approved beats into comic scripts, storyboards, and finished pages at `/story/`,
and into campaign mission briefs and playable scenarios. Do not publish the
outline verbatim as a lore article or discard it when adaptation begins. It
continues to hold continuity, decisions, and unresolved work.

Record planned events in the production outline now. Add a public article's
`History` section when the relevant event first appears in a released comic
installment or campaign mission; do not wait for both adaptations. State the
spoiler scope and link to the released installment. Keep the article's current
status and tense consistent with the story point its history covers.

Select events that matter to each subject. Gantry's history can describe its
attack and evacuation, while Kaveri's briefly records its part in the rescue.
Character histories hold their relevant consequences. Do not copy the full
season chronology onto every related page, add empty History sections, or
invent calendar dates before the chronology is established. Ordinary Markdown
sections and links are sufficient; there is no separate timeline system.

Unpublished means absent from the website, not secret. The production sources
remain readable in the repository.

## Ships as a recurring cast

Name every ship deliberately authored into the shared story, not only the main
ship and large opponents. Important vessels have identities and relationships
comparable to stations. A name introduced just before a destruction is not a
substitute for familiarity: show useful work, recurring visits, recognizable
crews, and the people who depend on a vessel.

This includes ships acting off-screen, such as the vessels that attack Gantry
before Kaveri arrives. Give them names and record their involvement in the shared
production outline, even if their names are not spoken in that installment.
A ship's identity and later appearances must not depend on whether the camera
showed its earlier action. Do not force names into dialogue just to register them.

Company articles own their naming conventions. Ship articles hold stable
identities and regular port associations. The unpublished season or episode
outline tracks whereabouts at specific story turns, departures, damage,
capture, and loss. Do not present a regular port as a current berth or infer
crew deaths from a destroyed hull. Redress and Windfall have approved
pirate-chosen names. Their trial business origin and ship financing live in the
[season outline](seasons/season-1.md#former-working-outfit), not the public
articles. Earlier names and operators remain open. These two pirate names do
not establish an automatic renaming rule for every sale or capture.

Designs, dimensions, equipment, service histories, exact berths, and additional
crew identities still need approval. Unassigned craft in the station concept
art are not automatically the newly named ships. Do not add a complete fleet
or biographies merely to fill out an operator's roster.

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
guide and is not registered or published. Season and scene outlines also remain
unregistered. Publish their adapted story, not the production documents; follow
the publication policy above for article histories.

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
Character portraits are likewise appearance studies, not fixed ages,
ancestries, clothing, or likenesses. They have no military rank insignia.

The original five crew portraits share a drawing template in
`../../../scripts/gen-lore-portraits.py`. Heads used by the comic are shared with
the illustration library; see its ownership notes for the current scope. The
five saved portrait assets remain unchanged.
Run `python3 scripts/gen-lore-portraits.py` from the repository root to render them,
or add `--check` to verify the saved SVGs without writing. Keep the generator and
its outputs together.

Reusable scene and design-sheet drawings live in
[`scripts/nova_illustration/`](../../../scripts/nova_illustration/README.md).
Each named ship has one unscaled shape model for scene and orthographic views.
Character poses are authored drawings, not automatic rotations of a portrait.
The current scene treatment reuses original frontal heads, with separate body
staging and light shadow planes. Ship proposals use
faceted plating and selected industrial fixtures as visual cues, not copied
engineering or gameplay specifications.
`colors.py` owns the shared palette; `styles.py` applies the color-only comic or
lore presentation without changing geometry. Public lore does not depend on task
artifacts.

Run `python3 scripts/gen-lore-designs.py` to regenerate Kaveri's and Ebro's
proposal sheets and Elena's portrait, or add `--check` to verify them. Those
sheets are external-form concepts, not game loadouts, measured plans, or approved
construction details. Other current images are directly authored SVGs.

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

The comic uses a broader full-color palette, with green as a recurring accent
tied to the CRT visual language. It is not tinted green throughout. Warm
interiors and skin tones contrast with cool space, with jade and mint accents.
Approved designs remain consistent between formats; the encyclopedia
treatment does not define physical paint, company livery, skin tones, or
planetary colors.

The exterior, branding, and portrait SVGs retain their underlying color values.
Earlier images use an embedded `encyclopedia-green` filter on the inner
`artwork` group. Shared-library exports apply the same channel treatment with
instance-specific filter IDs, so several drawings can share one SVG safely.
Keep the filter on an inner artwork group, not the root SVG: Firefox does not
apply it there. Lettering and diagram labels stay outside the treated artwork.
A full-color export omits that group's `filter` attribute without changing the
drawing or its base colors. Those colors remain working art choices, not approved world
specifications. The unassigned station concept and reference diagrams already
use the green palette and do not need this treatment.
