# Four-page opening: clean-line draft

**Frozen review.** The accepted SVGs and HTML are retained unchanged. The old
source is `generate.py.txt`; commands below describe that historical workflow.
Maintained sources now live in `web/src/comics/season-1/episode-1/`. Use
`npm --prefix web run serve` and the real `/story/` reader for new work.

Open [index.html](index.html) directly in a browser. No server, website build,
or network connection is needed. This remains a private review board, not a
registered comic or a second maintained story specification.

The user requested this restyle after accepting the frontal-face and industrial
ship treatment. Compare the [previous four-page version](../proof/before-clean-line-opening/index.html)
or the [accepted three-shot study](../clean-line-study/index.html).

The requested [expression trial](../expression-study/README.md) now uses Rina's
amused expression and Jonah's wry half-smile on page 1. All other appearances
retain their original features. The user accepted these variants and requested
their commit. This expression choice remains limited to the coffee exchange.
The later script review approved 2078 and a small page-2 action revision.

## What changed

- Closer human framing with the original frontal heads, light shadow planes,
  visible necks, fine clothing lines, and separately staged bodies.
- One shared Kaveri and Ebro model in exteriors, the window shot, and lore sheets.
- Cropped work surfaces, service pipes, machinery, and window structure instead
  of distant figures in empty rooms. Elena retains the open working gesture.
- Full comic color: warm interiors, cool space, industrial gray, varied clothing,
  and green accents. No grain overlay or all-green presentation.

The four page titles and ten panels stay. The 130 existing spoken words remain,
with Samir's four-word residential supply report added on page 2. His check
replaces the screen-only panel while Leila examines the pump with her hands.
Samir reuses his original portrait identity, not a new likeness.

Baikal's first-exterior card now says 2078. Kaveri's card at the move aboard says
"Later that day." The line stops safely; residential
supplies remain normal. Elena connects Ebro's EarthWorks order to Foundation.
The draft ends with departure for Aquila. No pickup, rescue, combat, pursuit,
or pirate viewpoint is added. Four pages are not an episode boundary.

The [shared production outline](../../../web/src/lore/seasons/season-1.md#work-and-a-costly-rescue)
remains the continuity source. The year is now approved; no day, month, moon,
orbit, dimensions, capacity, engineering plan, or game interaction is established
by this art.
Apparent ages, clothing, colors, interiors, and scene props remain proposals.

## Review

Use the page links, Previous / Next, or arrow keys when a control has no focus.
Contact sheet compares the pages. Art only hides speech but keeps physical
signs and orientation cards. Printing includes all four landscape pages.

Use the full-size SVGs to zoom and the text transcripts on small screens.
The scaled landscape overview is not a final mobile reading experience.
[Restyle review and checks](../RESTYLE-REVIEW.md) records the accepted staging.
[Expression review and checks](../EXPRESSION-REVIEW.md) records the accepted pair.
[Episode feedback and checks](../EPISODE-FEEDBACK.md) records the current revision.

## Source and regeneration

`generate.py` owns composition, local backgrounds, the pump and record props,
lettering, transcripts, and the board. It imports the maintained
[illustration library](../../../scripts/nova_illustration/README.md) for recurring
heads, bodies, ships, scenery, colors, and SVG primitives. The mug-and-hand
source is task-local in `../opening_props.py`, also used by the comparison study.
It is not a series-wide prop asset. Public exporters never import task files.

The library now supplies all five crew heads and Elena's. Leila and Samir use
layered inspection forearms, with the scene's equipment between body and hands.
The five existing public crew portraits remain unchanged. The legacy portrait
template colors have not been migrated.

From the repository root:

```bash
python3 tasks/20260908-161328/comic-opening-poc/generate.py
python3 tasks/20260908-161328/comic-opening-poc/generate.py --check
node tasks/20260908-161328/proof/inspect-opening-feedback.mjs
python3 tasks/20260908-161328/proof/check-episode.py
```

The generator writes only `index.html` and `page-01.svg` through `page-04.svg`.
The Chromium check writes current evidence under `../proof/episode-feedback/`
and stops its owned browser. Older proof writers belong to their recorded
revisions; do not run them against this revision. Do not hand-edit generated outputs or overwrite the frozen
comparison files. Keep this draft outside public discovery and website copying.
