# Opening comic visual PoC

An unpublished four-page color and composition study. Open `index.html` in a
browser. No server, website build, or network connection is required.

This is a review board, not a second comic reader or a registered story. The
public archive still contains the Reader demo. Do not add this directory to the
website's copy rules or comic discovery roots.

## What this tests

- A broader palette: jade and mint accents, indigo space, warm ochre light,
  coral interiors, and natural skin tones. No all-green filter or CRT bezel
  covers the artwork.
- A working community before the crisis. People, machinery, and exteriors share
  the page; this is not a sequence of interface cards.
- Fuller conversations that show professional judgment and working
  relationships, readable speech balloons, and room for silent images.
- Cinematic-style location/time/context cards inside Baikal on page 1 and
  aboard Kaveri on page 4. Relative day labels are draft staging; the calendar
  date is explicitly marked TBD. No moon is assigned to Baikal.
- Four draft pages through Kaveri's departure from Baikal, not a fixed episode
  boundary. The rescue and the Aquila encounter are outside this sample.

The shared production source is
[`season-1.md`](../../../web/src/lore/seasons/season-1.md#work-and-a-costly-rescue).
The page beats are Baikal at work, the safely isolated processing line, the
replacement pickup and Ebro commitment, then departure for Aquila. Page 3 names
Foundation on the delivery record and connects the order to EarthWorks through
Elena's explanation, without a separate exposition scene.

## Review

Use the page links or Previous / Next. Left and right arrows also work when a
control does not have focus. Contact sheet compares the compositions. Art only
hides the speech balloons, not physical signs or the location/time cards.

Open a full-size SVG to zoom or inspect individual details. Each page has a
plain-text dialogue transcript. On a small screen the page overview is for
composition; use zoom or the transcript to read, rather than treating scaled
lettering as the final mobile reading experience. Printing includes all four
pages in landscape.

Look for these decisions, rather than polish:

1. Does green still feel like Nova when the world has other colors?
2. Does Baikal look inhabited and useful, rather than bleak or abandoned?
3. Do the people and their conversations belong in the same visual world as
   the ships?
4. Does the quiet opening earn its space? Would a different panel rhythm read
   better?
5. Do the fuller conversations stay readable, and do the cards make changes of
   place and time clear without covering the people?

## Provisional, not canon

All drawn ship and station geometry, relative scale, orbital framing, interiors,
faces, apparent ages, clothing, colors, signs, and dialogue are concepts. No
measurements or travel schedule are established. The in-scene machine displays
are illustrative props, not specifications for game UI.

Only Kaveri and Ebro appear as ships. Kaveri is unarmed. The warm corridor is a
habitation concept; the exterior's ring does not approve a final Baikal design.
No attack, pursuit, or pirate viewpoint is added. Residential supplies stay
normal while the industrial line is stopped. The protagonist crew remains
competent.

The existing portrait source supplies the five Kaveri crew head studies without
its encyclopedia filter. Elena's head and all body poses are new provisional
studies. These do not settle public character designs. The mug and game-night
notice are draft staging, not separately approved lore.

## Source and regeneration

`generate.py` owns the illustrations, lettering, transcripts, and review board.
It uses only the Python standard library and the existing portrait source at
`scripts/gen-lore-portraits.py`. It does not modify that source or its assets.
Do not edit the generated HTML or SVGs by hand.

From the repository root:

```bash
python3 tasks/20260908-161328/comic-opening-poc/generate.py
python3 tasks/20260908-161328/comic-opening-poc/generate.py --check
```

The generator produces `index.html` and `page-01.svg` through `page-04.svg`.
Keep these outputs with the source so the study opens directly. Accepted
compositions can later be adapted into typed comic pages and their artwork;
this PoC does not change the public renderer or its content contract.
