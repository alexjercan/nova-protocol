# Visual and audio candidates for the progression spike

Research only, 2026-09-25. No asset was downloaded, imported or placed in a game build for this spike. Licenses of *specific packs* matter; a marketplace, site or 'free' label alone is not a license. Sources below are project records and original creator pages checked with `curl`. Before shipping any newly acquired file, preserve its exact source/version/license and test the export/size/appearance in Nova; approval and content integration are separate work.

## Prefer proven local material first

| Candidate and source | What it could test | Limitation / route |
| --- | --- | --- |
| Existing authored ship skin, sections, greebles, textures and UI icons (`credits/CREDITS.md:11-43`) | Sectional salvage, a visibly fitted upgrade, a dock terminal as a ship-screen layer | Base GLBs and original assets already ship; reuse without a new license. A section mesh is not an inventory item or a runtime refit. |
| Project-built sounds (`credits/CREDITS.md:16-25`, `scripts/gen-world-sfx.py`, `scripts/gen-ui-sfx.py`) | Extract/claim/denial/dock feedback | Existing cues were not authored for the proposed transactions; audition and generate new cues rather than relabel blindly. |
| Kenney Space Kit, already in `art/kenney-space-kit/` (`art/README.md:15-18`, `credits/CREDITS.md:60-64`, `credits/licenses/Kenney_Space_Kit_License.txt`; [original pack](https://kenney.nl/assets/space-kit)) | Rapid static station/support-module silhouettes | CC0, but existing kit is **not** used by the base game: only Ledger ships cut it into parts. Geometry/style, docking clearance and size need a playable check; do not simply copy source art under `assets/`. |
| Quaternius Ultimate Spaceships, already source-baked (`art/README.md:24-35`, `credits/licenses/Quaternius_Ultimate_Spaceships_License.txt`; [original pack](https://quaternius.com/packs/ultimatespaceships.html)) | Cheap derelict silhouette comparisons | CC0; repo bakes flat-Kd geometry for comparisons. A decorative whole model is not a validated multi-section ship, AI hull or dockable station. Source atlas zip is not committed. |
| Fertile Soil Spaceship Blocks Collection (`art/README.md:19-23`) and generated part candidates (`art/README.md:36-42`) | More varied creator-authored ship sections or fixtures | Local record says CC0 from creator page, but the zip has no embedded license. Candidates are NOT runtime content; preserve creator-page license proof before promotion. |
| ambientCG and Poly Haven rock candidates (`art/README.md:62-86`; [ambientCG license](https://docs.ambientcg.com/license/), [Poly Haven license](https://polyhaven.com/license)) | Different readable rock silhouettes/material cues | Both candidate sets recorded as CC0; JPEG maps were converted for Nova's PNG pipeline. Existing candidate color maps do not prove mineable resource type or scanning. Inspect close and far against dark space. |
| Unassigned station and supply diagrams (`web/src/assets/lore/industrial-station-concept.svg`, `usable-supply.svg`; `web/src/lore/society.md:53-75`, `economy.md:25-53`) | UI mood board and near-future supply vocabulary | Lore art is conceptual, not a shipped station blueprint, 3D model or permission to assign a name/scale. Use it as internal reference, not a new canon asset. |

## One outside candidate with an explicit source license

[KayKit Space Base Bits 1.0](https://github.com/KayKit-Game-Assets/KayKit-Space-Base-Bits-1.0): the creator's [README](https://github.com/KayKit-Game-Assets/KayKit-Space-Base-Bits-1.0/blob/main/README.md) names 48+ low-poly modular models with OBJ/FBX/glTF and commercial use, and [LICENSE.txt](https://github.com/KayKit-Game-Assets/KayKit-Space-Base-Bits-1.0/blob/main/LICENSE.txt) records CC0. Could mock a dock backdrop or service-module grouping. Not installed; importing needs an exact revision, credit record, Blender/export/material check, collision/size budget and a rendered aesthetic comparison to Nova's harder-edged authored art. 'Ready GLB' and 'shipping-ready' are **not** established: the creator lists glTF, not a tested Nova GLB. No need to add it if existing local geometry suffices.

## Excluded or unverified suggestions

- A third-party 'Kenney Modular Space GLB' conversion is not equivalent to the original Space Kit or a verified direct creator pack; do not download or ship it without a derivative/license and geometry audit.
- OpenGameArt, Sketchfab and Freesound aggregate mixed per-upload licenses. No specific model/sound with pinned uploader, revision and license was vetted; these are search venues, not candidates approved for use.
- ZapSplat was wrongly described in an earlier agent report as universally CC0. That claim is **not supported**; do not source from it under a CC0 assumption. Google Fonts also licenses individual fonts, not every typeface uniformly. Nova already credits and ships Iosevka Term under SIL OFL 1.1 (`credits/CREDITS.md:91-95`).
- No downloaded third-party game screenshots, UI art or franchise likeness can be used in Nova's production mockups just because they help explain a competitor.

## Small visual proof before adopting anything

Start with original ship parts, generated greebles, local rock textures and the existing phosphor palette. Compare a dock/rock field at player range, scanner range and on the smallest target screen; judge docking-port recognition, clutter, and a salvaged part's silhouette. For an external model, record creator/source URL, exact release/revision, on-disk license, transformations, atlas/textures, target GLB size, attribution placement and rights for both web/native redistribution in `credits/CREDITS.md` and `credits/licenses/` when actually imported. Do not assert readability or performance from a catalog thumbnail.
