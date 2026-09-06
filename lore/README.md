# Lore encyclopedia

The world reference for Nova Protocol, presented as a CRT encyclopedia with
mdBook. The cover and its station illustration are at `src/index.md`.

## Reference structure

- [World overview](src/setting.md) records the approved setting, shared corporate
  and institutional background, and open decisions.
- [Browse subjects](src/contents.md) groups references by places, companies and
  institutions, military and security, society and economy, and ships and
  technology.
- `src/places/` contains location articles, beginning with
  [Keystone](src/places/keystone.md).
- `src/organizations/` contains named company and institution articles,
  including [EarthWorks Industrial](src/organizations/earthworks-industrial.md)
  and [Farspan Logistics](src/organizations/farspan-logistics.md).

Add individual companies, places, and institutions as their identities are
approved. Create new subject directories and navigation sections only when they
have articles. Until then, the subject directory links to relevant background
in the world overview. Do not create empty chapters or supply missing names
and specifications for completeness. Do not recover earlier lore from Git.

## Narrative and shared canon

The main narrative belongs in the [web comic](../web/src/comics/), not in a
sequence of encyclopedia chapters. Readers should understand the comic without
first reading this reference. Article links provide optional depth. Short
anecdotes or document excerpts can illustrate an article without becoming the
main story delivery format.

The encyclopedia and comic share one canon. When an approved story decision
establishes a world fact, update the relevant reference instead of maintaining
a separate version of the setting. No protagonist, central conflict, or plot
has been chosen yet.

Game campaigns can adapt selected events or follow another perspective. Comic
chapters do not need to correspond one-to-one with missions or explain game
mechanics.

## Technical reference

[Technical specification](TECHNICAL_SPEC.md) records mechanics, configured
values, derived estimates, and implementation gaps at its stated implementation
revision. It is a working reference, not a book chapter or story canon. Recheck
the relevant code before relying on a mechanic for a campaign.

## Illustrations

Current images are directly authored SVG files in `src/images/encyclopedia/`.
The cover has its own station concept, not repeated in the articles. Place
articles carry separate exterior studies, and company infoboxes carry emblem
and wordmark proposals. Caption relationship diagrams as references and artwork
as visual concepts. Concepts do not establish a location, design, scale,
construction history, or official branding without separate approval.

Use direct SVG for diagrams, emblems, and simpler illustrations. Use a script
when repeated geometry or regeneration provides a clear benefit; do not build
a generator for every image. For a generated image, commit both its generator
and output, and change the generator rather than hand-editing the output.

### Color and presentation

Use [the cover illustration](src/images/encyclopedia/industrial-station-concept.svg)
as the encyclopedia's palette reference: green-black backgrounds, sage and
olive surfaces, pale green highlights, and restrained amber accents. This
applies to both place illustrations and company identity proposals.

The comic's art direction is restrained full color, with green as a recurring
accent tied to the CRT visual language. It is not tinted green throughout.
Approved designs remain consistent between formats; the encyclopedia treatment
does not define physical paint, company livery, skin tones, or planetary colors.

The exterior and branding SVGs retain their underlying color values. An
embedded `encyclopedia-green` filter on the inner `artwork` group supplies the
book's presentation, including when the image is opened directly. Keep the
filter off the root SVG: Firefox does not apply it there. A full-color export
can omit the artwork group's `filter` attribute without changing the drawing
or its base colors. Those colors remain working art choices, not approved world
specifications. The original cover and reference diagrams already use the green
palette and do not need this treatment.

Do not add or run book tests.
