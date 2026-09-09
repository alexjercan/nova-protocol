# Clean-line style study

Open [index.html](index.html) directly in a browser. This is an unpublished
three-shot trial of clean-line graphic realism. Its SVGs remain unchanged after
acceptance. Compare the [previous four-page PoC](../proof/before-clean-line-opening/index.html)
or the [restyled four-page opening](../comic-opening-poc/index.html).

## Selected for trial

The user accepted the frontal/plated revision as the working visual direction.
The goals are closer character acting, useful objects, and occasional wide
views of the working world. Keep the broader palette and green accents.

- Deliberate outlines, with lighter lines for facial features and small details.
- Flat base colors and limited shadow planes on people and machinery.
- Gradients only for sky and atmosphere. No overall grain. Comic art uses full
  color by default; the Lore colors switch previews the shared green treatment
  without changing geometry, speech balloons, or page labels.
- Palette constants live in `scripts/nova_illustration/colors.py`. Character
  poses share named skin/clothing colors; ships share material roles.
- The user rejected the angled faces and chose the original forward-facing
  portrait treatment. Keep the trial's zoom, frames, backgrounds, body staging,
  necks, and linework. Elena's eyes are open and face the reader.
- The user likes the ship-sheet format, but wants game-like plating and
  industrial greebles. Facets change the outline; fixtures are not a substitute
  for a plated hull.
- Background detail follows the subject: an object, window frame, work surface,
  or silhouette, rather than a fully furnished room in every panel.

## Three shots

| Study | Existing material | Test |
| --- | --- | --- |
| [Close-up](01-close-up.svg) | Page 03's borrowed-mug joke | Elena's face, eyeline, and expression, with Jonah's hand and mug in the foreground. |
| [Window conversation](02-window.svg) | Page 03's Aquila pickup exchange | Frontal heads on the same working gesture and listening shoulder frame the waiting Ebro. The window gives depth and connects people to work. |
| [Exterior](03-exterior.svg) | Page 04's departure | Kaveri, Baikal, Ebro, and Saturn in the same line-and-shadow language, with construction detail concentrated on the nearer hull. |

The sample order is for comparison, not chronology. The mug joke still follows
the pickup discussion in the opening. All spoken words come from the existing
PoC. Cream balloons omit visible speaker labels in this trial; the transcripts
still identify speakers. This tests whether acting and balloon placement carry
the exchange without an illustrated-briefing treatment.

Only Kaveri and Ebro appear as ships. Kaveri is unarmed; its folded recovery arm
is not a weapon. The departure precedes the Aquila pickup, so the work deck has
no replacement assembly aboard. No attack, pursuit, route, year, or moon is
added. These images neither introduce nor replace the opening's location/time
cards; those remain in both four-page versions.

## Not yet canon or finished art

All appearances, clothing, physical colors, hull construction, scales, light
sources, interiors, and orbital framing remain concepts. The drawing coordinates
are illustration coordinates, not meters or engine units. This is not a ship
specification, station layout, or simulation diagram.

Elena's new public portrait uses the same close-up drawing with lore colors.
The original five crew portraits and earlier draft's heads remain unchanged.
The accepted treatment now carries into the four-page draft at the user's
request. Review its page rhythm and staging there; this comparison does not
reopen the rejected angled-face treatment.

## Regeneration and review

`generate.py` owns scene composition, dialogue placement, the three SVGs, and
the offline review sheet. `web/src/comics/season-1/episode-1/art/opening_props.py` now supplies the same incidental
mug-and-hand drawing to this study and the four-page draft. It uses shared
colors but remains task-local, not a series-wide library asset.
The scene generator imports the maintained
[illustration library](../../../scripts/nova_illustration/README.md), not the old
PoC. That library owns the shared palette, poses, ship models, and color
presentation. Change the sources, not generated outputs.

Kaveri and Ebro now each have one solid shape model for scene views and green
CAD-style lore sheets. The first separately drawn style-study hulls are retained
only in `../proof/clean-line-initial/` screenshots. The new forms take cues from
the game's sloped plating, framed vessels, louvres, fin banks, hatches, handling
gear, and restrained hazard paint. They are not imports of game loadouts. Baikal remains a reusable 2D silhouette for now.

From the repository root:

```bash
python3 tasks/20260908-161328/clean-line-study/generate.py
python3 tasks/20260908-161328/clean-line-study/generate.py --check
node tasks/20260908-161328/proof/inspect-clean-line.mjs
```

The browser check needs Chromium on PATH. Its images and report go to
`../proof/clean-line/`; its temporary browser profile stays outside the
repository. Keep the earlier baseline and revised-PoC evidence unchanged.
Pass a website build's parent directory as the browser script's first argument
when checking the built `/nova-protocol/` articles too. Without that argument,
it checks only local study files and standalone lore exports.

The review sheet includes full-size SVG links, an Art only toggle, and text
descriptions/transcripts. On a small screen, use SVG zoom or the transcript;
scaled landscape art is a composition overview, not a final mobile reader.

These scene files are outside public comic discovery and all website copy
inputs. The separately exported Kaveri/Ebro design sheets and Elena portrait
belong in their public lore articles as explicit proposals. Public exports use
only the maintained library, never this task's scene files.
Keep later experiments and review evidence with the same story task. The
[shared production outline](../../../web/src/lore/seasons/season-1.md) remains
the continuity source; this study is not another story specification.
