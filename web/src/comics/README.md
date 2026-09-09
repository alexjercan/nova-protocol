# Comic authoring

This directory owns comic metadata, scripts, and panel art. The real website
reader is also the local review surface. Shared faces, ships, scenery, and
colors stay in [`scripts/nova_illustration/`](../../../scripts/nova_illustration/README.md).

```bash
npm --prefix web run serve
```

Open `/story/` at the printed loopback URL. Normal local development includes
labelled drafts. Source edits rebuild and reload the current page fragment.
`scripts/serve-web.sh` uses the same server alongside the game and mod portal;
its `--release` option optimizes the game, not story publication.

`npm --prefix web run build` is released-only, even in webpack development
mode. Serving with `--mode production` also excludes drafts. Local serving
uses `web/.cache/story/development/site`, never stale files from `web/dist`.
There is no separate preview command, browser editor, or demonstration episode.

## Season and episode pages

`/story/` lists selected seasons in reading order, with no episode entries.
Each season has one clickable illustrated row, its title and summary, and the
selected episode count. Local draft counts are labelled. Click a season to open
`/story/<season>/`, which lists only that season's episodes in authored order.

The season page has a **Story** return link, a heading, and one clickable row
per episode. Each episode has one thumbnail; a draft shows its illustrated and
complete page counts. There are no Preview, Start, Latest, or Season contents
shortcuts, and no repeated season cover above the episodes.

Only build-selected seasons and episodes get pages. The reader's return link
uses the season title, such as **Season 1**, and leads to its episode list.
**Pages** opens the page chooser. Panels, Transcript, native SVG exports, and
page/episode navigation remain available.

## One script, one file per page

```text
season-1/
  comic.json
  episode-1/
    episode.json
    episode.ts
    NOTES.md
    pages/
      page-01.ts
      page-02.ts
      ...
      page-18.ts
    art/
      opening.py
      aquila.py
      distress.py
      props.py
      opening_props.py
```

`episode.ts` imports the pages explicitly, in reading order. Each page owns its
action, dialogue, story labels, and composition. Python owns only scene artwork.
`NOTES.md` holds production constraints, not a second copy of the dialogue.
There is no episode-specific `generate.py` or maintained Markdown script.

The [page language](comic-script.ts) keeps words separate from placement. This
excerpt omits the console's screen labels; see the complete
[page-two source](season-1/episode-1/pages/page-02.ts).

```ts
panel("2b", {
    art: scene("residential-console"),
    action: "Samir checks residential supplies at the processing console.",
    dialogue: [
        say("supply-normal", "Samir", "Residential supply is normal."),
    ],
    lettering: {
        "supply-normal": balloon({
            at: [16, 18],
            width: 307,
            tail: [373, 167],
        }),
    },
});
```

A `page({ id, title, purpose, panels, layout })` places those panels. Its layout
is keyed by panel id, with `at: [x, y]` and `size: [width, height]`. These are
illustration coordinates, not physical measurements. Panel art, text, and
balloons share the scene's local coordinate system; page placement scales them
together without deforming them.

- `say(id, speaker, text)` contains a whole spoken line or paragraph. Speaker
  labels are literal text, including comms and off-panel qualifiers.
- `balloon(...)` owns position, width, tail endpoint, and tail side. An absent
  side means `bottom`. An absent or empty `breakAfter` means automatic browser
  wrapping. Deliberate breaks use ascending word counts, not copied text.
  Existing accepted lettering retains these breaks; remove them to use fully
  automatic wrapping. Stale or invalid break indices fail the build.
- `label(id, text, options)` owns readable screen/sign text and its styling.
  `slot` refers to a named blank insertion point in the SVG. `at`, `size`, and
  `color` are required. Default styling is normal weight, zero letter spacing,
  and start anchoring. Color names come from the shared Python palette.
- `location(id, { name, place, date, note, at, width })` supplies a narrative
  orientation card. The engine draws its frame and text.
- `scriptPage(...)` and `scriptPanel(id, action, dialogue)` keep unillustrated
  pages in the same maintained script, with no fake scene ids or geometry.
  They do not enter reader manifests. Illustrated pages retain their actual
  script page numbers, including when work is completed out of order.

Dialogue ids must match lettering keys exactly. Page placement must account for
every panel exactly once. Missing fields, unknown scene/slot/color references,
unlettered slots, duplicate ids, and invalid geometry fail the build. Empty
label/card arrays mean none are present; the `panel` helper supplies them when
omitted. Silent panels still have authored action and a useful derived transcript.

Episode one has eighteen script pages. Ten are illustrated; it remains a draft.

## Python panel art

Art modules export `SCENES`, a mapping from scene ids to drawing functions.
Each function returns `Scene(width, height, art)` from
`nova_illustration.scenes`. The shared builder owns imports, registration,
validation, SVG wrapping, and exports. It calls only requested scenes.

Use `text_slot("status")` where a screen label belongs in painter order. The
slot has no words, font settings, or page coordinates. TS inserts the label
there, including under foreground hands or inside a rotated equipment group.
Python does not draw dialogue and then strip it out.

Recurring subjects stay in the shared illustration library. Incidental props
stay under the episode's `art/`. Raw scene SVGs contain art and blank text slots;
lettered panel exports contain the story labels, cards, and dialogue as well.

## Metadata and publication

All metadata fields are required; unknown fields fail.

- `comic.json`: positive unique `sequence`, `title`, `summary`, and a nonempty
  ordered `episodes` array. Every episode directory must be registered. Directory
  names are the season and episode route ids.
- `episode.json`: `title`, `summary`, `publication` (`draft` or `released`),
  positive complete `pageCount`, `source` (a local TS filename), `art` (explicit
  `art/*.py` module paths), `cover` (a scene id), and `coverAlt`.
- Script length must equal `pageCount`. A selected episode needs illustrated
  pages. A released episode must have every page illustrated.
- Released episodes form a prefix of global reading order. A later episode
  cannot release before an earlier draft. A season may contain both.

The season cover comes from its first selected episode. An excluded draft can
never supply a public cover dependency. Release follows completion and approval:
change `publication`, then build normally. No source move is needed. Comic
publication is independent of campaign readiness.

## Publishing the comic

Comic releases do not change the game version. A `comic-*` tag marks the exact
source snapshot of the whole released library, not an episode selector.
`comic-s01e01` and `comic-season-1` are examples. Keep tags immutable; use a new
tag such as `comic-s01e01-r2` for a correction.

1. Finish and review the episode's art, text, navigation, and exports.
2. Set its `publication` to `released`. Every authored page must be illustrated.
   A season batch marks each finished episode released. Reading order remains a
   global released prefix; drafts cannot appear between released episodes.
3. Run `npm --prefix web run test:deploy` and
   `PUBLIC_PATH=/nova-protocol/ npm --prefix web run build:story`.
4. Review `web/dist-story/story/`. Commit the approved sources, then create and
   push the chosen `comic-*` tag. This starts `deploy-comic` automatically.
5. Verify `/story/`, the episode, and `/story/release.json`. The record contains
   the tag and exact source commit. Creating a tag does not change publication
   fields or make an incomplete episode publishable.

`deploy-comic` can also run manually with an existing comic tag, including to
restore an earlier library snapshot. It checks out that tag, builds only the
comic, and replaces only `story/` on `gh-pages`. No game, wiki, News, developer
book, or mod-portal build is part of that workflow. Repository-owned reader
scripts, images, favicon, and font assets are inside `/story/`; the shared
Google Fonts import remains external. This is not an offline-site guarantee.

The ordinary Pages workflow uses `build:site` and preserves the independently
published `story/`. Both publish jobs share a lock, fetch the latest `gh-pages`
tip after acquiring it, and push without force. Concurrent outside changes fail
instead of being overwritten. Both explicitly request and wait for a Pages
build: a push made with `GITHUB_TOKEN` alone does not start one.

### First rollout and hosting

Keep Pages configured to deploy from the **gh-pages branch root**, not the
Actions-artifact source. The workflows require `contents: write` and
`pages: write` in their publish jobs; the build jobs have read-only tokens.
`CNAME` is preserved, and `.nojekyll` remains present.

First publish an empty-library bootstrap tag, such as `comic-reader-1`, from a
commit containing this workflow. Drafts stay excluded. Do this before the first
site-only deployment: an older Story page may still depend on root-level reader
files. The publisher refuses to replace the surrounding site until a comic
release record establishes the new ownership boundary. Run the full-site
workflow from current `master`; old workflow revisions lack this protection.

GitHub serializes running publishers but does not promise FIFO execution of
pending jobs. Wait for a release to finish before queuing another. A failed
Pages request can be retried by rerunning the workflow; unchanged artifacts do
not create another branch commit.

## Shared build and review engine

`web/comic-sources.js` validates metadata and selects publication BEFORE
importing any episode TS or Python module. `web/read-comic-script.js` evaluates
only a selected entry and its explicit imports in a fresh process. The shared
`web/comic-script-build.js` validates its data and derives transcripts, Markdown,
and reader definitions. `web/build-comics.py` exports panel art only.

Generated files live under
`web/.cache/story/{development,release}/generated/<season>/<episode>/`.
`SCRIPT.md` there is a derived reading copy of every TS page. It is not copied
to the site. Unchanged generated files retain their timestamps; stale files
are removed. The build embeds only selected illustrated page definitions and
emits only referenced images under `/story/assets/<season>/<episode>/`.
Excluded episode code, future script pages, and source paths never enter the
reader bundle or source maps. There is no recursive page-module import or
whole-directory asset copy.

Page/source changes within existing routes update automatically. Adding or
removing episode/season routes requires a server restart; watch builds fail
instead of emitting broken links. Restart after changing build-engine JS too.

The browser imports restricted SVG, composes page frames and panels, inserts
story labels, and measures dialogue with the actual font. Raw SVG exports are
also checked for executable or external content at build time. **Open image**
exports the displayed full page. Compact **Panels** and **Transcript** toolbar
buttons open scrollable modals, without reserving space below the page. Panels
links to individual art SVGs and lettered panel SVGs. Close or Escape dismisses
a modal and restores focus to its button; the background stays inert while it
is open. All composed exports use the reader's measured layout, not a second
export algorithm.

Automated browser checks inspect measured text overflow, panel/page bounds,
balloon collisions, and face-contour overlaps. There is no Layout checks control
or diagnostic overlay in the reader. Hair, props, and composition still need
visual inspection. Draft **Art only** hides speech, not story signs or cards.
The phosphor shell never tints full-color artwork. Phone fitting is a landscape
overview; native panel/page zoom and transcripts provide detail.
