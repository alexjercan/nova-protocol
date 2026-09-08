# Four-page opening: clean-line draft

Open [index.html](index.html) directly in a browser. No server, website build,
or network connection is needed. This remains a private review board, not a
registered comic or a second maintained story specification.

The user requested this restyle after accepting the frontal-face and industrial
ship treatment. Compare the [previous four-page version](../proof/before-clean-line-opening/index.html)
or the [accepted three-shot study](../clean-line-study/index.html).

## What changed

- Closer human framing with the original frontal heads, light shadow planes,
  visible necks, fine clothing lines, and separately staged bodies.
- One shared Kaveri and Ebro model in exteriors, the window shot, and lore sheets.
- Cropped work surfaces, service pipes, machinery, and window structure instead
  of distant figures in empty rooms. Elena retains the open working gesture.
- Full comic color: warm interiors, cool space, industrial gray, varied clothing,
  and green accents. No grain overlay or all-green presentation.

The four page titles, ten panel beats, 130 spoken words, display/record text,
and Baikal/Kaveri orientation text stay. Baikal's card is on the first exterior
picture; Kaveri's stays at the move aboard. The line stops safely; residential
supplies remain normal. Elena connects Ebro's EarthWorks order to Foundation.
The draft ends with departure for Aquila. No pickup, rescue, combat, pursuit,
or pirate viewpoint is added. Four pages are not an episode boundary.

The [shared production outline](../../../web/src/lore/seasons/season-1.md#work-and-a-costly-rescue)
remains the continuity source. No calendar year, moon, orbit, dimensions,
capacity, engineering plan, or game interaction is established by this art.
Apparent ages, clothing, colors, interiors, and scene props remain proposals.

## Review

Use the page links, Previous / Next, or arrow keys when a control has no focus.
Contact sheet compares the pages. Art only hides speech but keeps physical
signs and orientation cards. Printing includes all four landscape pages.

Use the full-size SVGs to zoom and the text transcripts on small screens.
The scaled landscape overview is not a final mobile reading experience.
[Restyle review and checks](../RESTYLE-REVIEW.md) records the current evidence.

## Source and regeneration

`generate.py` owns composition, local backgrounds, the pump and record props,
lettering, transcripts, and the board. It imports the maintained
[illustration library](../../../scripts/nova_illustration/README.md) for recurring
heads, bodies, ships, scenery, colors, and SVG primitives. The mug-and-hand
source is task-local in `../opening_props.py`, also used by the comparison study.
It is not a series-wide prop asset. Public exporters never import task files.

The library now supplies the needed Leila, Rina, and Tomas frontal heads as well
as Jonah and Elena. The five existing public crew portraits remain unchanged.
Samir's head and the legacy portrait template colors have not been migrated.

From the repository root:

```bash
python3 tasks/20260908-161328/comic-opening-poc/generate.py
python3 tasks/20260908-161328/comic-opening-poc/generate.py --check
node tasks/20260908-161328/proof/inspect.mjs
python3 tasks/20260908-161328/proof/check-opening-restyle.py
```

The generator writes only `index.html` and `page-01.svg` through `page-04.svg`.
The Chromium check writes current evidence under `../proof/first-panel-card/` and stops
its owned browser. Do not hand-edit generated outputs or overwrite the frozen
comparison files. Keep this draft outside public discovery and website copying.
