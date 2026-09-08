# Frontal faces and industrial plating

## User direction

The user accepted the clean-line trial's zoom, frames, camera placement,
backgrounds, body staging, necks, and linework. The angled faces did not work.
Restore the original forward-facing humans, with light shadows and detail lines.
Keep the green lore treatment, including Elena's portrait shader.

The ship-sheet concept and layout work. The hulls need to look closer to the
game: plating changes the outline with slopes, ridges, and recesses. It is not
just greebles on cubes. Use industrial fixtures and restrained accent paint.

This follows the earlier decisions to maintain one model per ship, shared
character sources, one palette, and color-only comic/lore schemes. Incidental
props such as the mug stay in the scene generator. No shared prop catalog is
needed yet.

## Results to review

[Open the three-shot study](clean-line-study/index.html): close-up, window
exchange, and departure, with Art only and Lore colors controls. The
[original four-page PoC](comic-opening-poc/index.html) remains unchanged.

- Elena and Jonah use their original frontal head contours and hair. Elena has
  open, forward-facing eyes. Light shadow planes and detail lines remain.
- The new necks, collars, clothing folds, working gesture, and listening shoulder
  stay. Scene transforms, camera views, frame geometry, backgrounds, dialogue,
  and balloon placement stay unchanged.
- Kaveri keeps the crew module, open work cradle, stowed handling arm, and paired
  engines. It gains a faceted lower hull, segmented cowls, sloping crew brow,
  industrial louvres, fin bank, access covers, ducts, and marked working edges.
- Ebro keeps three exposed framed vessels, with plated support structure,
  tapered crew module, fittings, service pipes, and edge markings.
- Gray plating, dark machinery, and restrained yellow details take more cues
  from the game. These remain proposed physical colors, not approved livery.

The study order is comparative, not a new chronology. No new story event,
weapon, ship specification, date, moon, or orbital relationship was added.

Public proposal exports retain the same sheet and portrait layouts:

- [Kaveri design sheet](../../web/src/assets/lore/kaveri-design-concept.svg).
- [Ebro design sheet](../../web/src/assets/lore/ebro-design-concept.svg).
- [Elena portrait](../../web/src/assets/lore/elena-ward-portrait-concept.svg).

The study remains private. The subject exports remain explicit public lore
proposals. The original five crew portrait SVGs remain byte-for-byte unchanged.

## Source decisions

Maintained library: [scripts/nova_illustration](../../scripts/nova_illustration/README.md).

- `faces.py` owns Jonah's and Elena's frontal contours, hair, and features.
  Jonah's original portrait exporter now imports its head from this source.
  Elena's head was promoted from the unchanged comparison draft; the library
  and public exporters never import task files.
- `portraits.py` owns the close and conversational body poses. Both Elena poses
  use the same frontal head. The rejected profile functions were removed.
- `colors.py` owns the new drawings' palette, including the retained face
  colors and industrial material roles. The other four original portrait
  heads and legacy template colors have not been migrated.
- `ships.py` owns one unscaled proposed model per ship. Scene and orthographic
  views do not have separate hull drawings. Plating and fixtures are geometry,
  not screen-space decorations added to selected views.
- `styles.py` is unchanged. It applies the same color-only transform to ships,
  people, and scenes. Speech and labels remain outside the filtered artwork.
- The scene generator owns framing, dialogue, and the incidental mug-and-hand
  drawing. Baikal, Saturn, and the star field remain unchanged library scenery.

More plating exposed a limitation of midpoint depth sorting: broad panels could
hide their own fixtures or paint over the open deck. The projector now uses
plane partitions, splitting overlapping surfaces and retaining only original
ink contours. Tests cover ordering and the absence of artificial ink seams.
This remains a small flat-surface illustration renderer, not a CAD kernel,
physical model, or game renderer. Inspect new forms and views.

## Local reference audit

Existing game screenshots inspected:

- `web/src/assets/greeble-catalog-industrial.png`: the industrial row's louvres,
  radiator fins, plate racks, crane, ducts, hatches, and edge accents.
- `web/src/assets/wiki-ships-damage.png`: sloped plating, ridges, recesses,
  exposed structure, and fixtures on the surviving skin. The damage itself is
  not added to the opening's intact ships.
- `web/src/assets/feature-autopilot.png`: plated ship silhouette in flight.

These existing 1920-by-1080 images and the recipe geometry were sufficient for
this pass. No new Rust capture or screenshot packaging was run. No existing
website media, wiki file, game recipe, generated game asset, or Rust code changed.

Read-only source references:

- `crates/nova_ship/src/sections/shell_skin.rs`, `boundary_heights`: the game
  tapers skin at open edges and retains rims against pockets. This informs the
  visual distinction; the comic does not duplicate the skin algorithm or units.
- `scripts/greeble-recipes/industrial_louvre.json`: sloping blades over a dark
  tray. No vacuum cooling-air behavior is inferred from the shape.
- `industrial_radiator.json`: a raised fin bank, not just painted parallel lines.
- `industrial_hatch.json`: layered mounting plate, cover, and accented handle.
- `industrial_hazard_band.json`: accents at working edges rather than across
  whole machines.
- `industrial_crane.json`: a restrained steel handling silhouette. Kaveri's
  proposed gear remains stowed, not a gun or copied game fixture placement.

Earlier references remain relevant: `scripts/section-part-recipes/` personnel,
cargo, and tank recipes; `scripts/thruster-shell-recipes/shell_twin.json`;
`gen-section-parts.py`; `nova_glb.py`; and `assets/base/ships/base.content.ron`.
The Utility Cutter is not automatically Kaveri. Drawing proportions are not
meters, build cells, engine units, cargo capacity, or physical specifications.

## Verification

Current evidence is under `proof/clean-line/`. The rejected angled-face pass
and earlier shared hulls remain as six comparison images under
[proof/clean-line-before-frontal](proof/clean-line-before-frontal/README.md).
The first separate hull drawings remain in `proof/clean-line-initial/`.

Affected checks:

- Thirteen library tests: face reuse, finite planar geometry, sloped plating,
  valid inputs, painter ordering, original ink contours, deterministic views,
  consistent orthographic extents, immutable palette, and color-only schemes.
- Regeneration checks for both current exporters, the original four-page PoC,
  and the original five public portraits.
- Lore tests: 28 pages and 724 source links. Comic and asset-namespace tests.
- Prefixed website build and exact copying of the three proposal assets.
- Three study SVGs, three lore exports, six desktop/mobile private views, and
  six built article views. Inspect lettering, navigation, keyboard activation,
  transcripts, image loading, and color-only switching.
- Source and publication checks: preserved scene layout and dialogue, unchanged
  baseline assets, no outside-scope changes, private drafts excluded, and
  demo-only public comic discovery.

The private preview needs no network. Built website pages retain the site's
existing Google Fonts requests; the browser check distinguishes those from the
private study. These are affected checks, not full website CI, Rust tests, or
a new game capture. No commit or deployment is part of this revision.

Reports: [unit tests](proof/clean-line/unit-tests.log),
[browser checks](proof/clean-line/browser-checks.json),
[website build](proof/clean-line/site-build.log), and
[source/publication checks](proof/clean-line/source-and-publication.json).

## User review

The user reviewed this revision and said: "it actually looks amazing".
This pass is accepted as the working visual direction: original frontal heads,
close framing, retained body staging and backgrounds, industrial plated hulls,
and color-only lore presentation. Keep this combination in later drafts.

This accepts the visual treatment, not unstated physical specifications,
chronology, or approval to replace the original four pages automatically.

## Next decision

Choose the next drafting scope before expanding the pose library or revising
the four-page opening. The calendar and location-precision decisions remain
open. No artwork changed while recording this review.
