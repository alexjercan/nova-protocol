# Store presence: the pitch, the trailer, the capsules, the copy

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.14.0, release, marketing, capture

## Goal

Make the game wishlist-able: decide the pitch, then produce the store assets
that the Steam page (`20260909-212935`) and the itch.io page
(`20260824-130004`) consume. Owner (2026-09-09): "think on how to market the
game and make it fun enough for people to wishlist it."

## Decisions

Owner decision, 2026-09-18:

- v0.14.0 is a quick initial release, not the open-world release.
- The v0.14.0 build is a free demo on itch.io.
- Owner update, 2026-09-21: keep the live itch.io `Name your own price`
  setting, with zero-cost download available. The project website links
  itch.io from the landing hero and the landing resource directory; the shared
  footer carries no links.
- Steam receives a Coming Soon page only. No Steam demo or game build ships.
- The source remains public on GitHub. Future Steam product pricing stays open
  and must not be implied by this page.
- Store publication happens after the v0.14.0 GitHub release. Asset production
  may finish before the tag.
- Approved store description and five feature sections are maintained in
  `PITCH.md`. Use that copy for this release without silent rewrites.

## The pitch first

Write it before any asset. One page in this task, `PITCH.md`:

- The hook in one sentence. Candidates from what the game already does:
  build a ship from sections and watch it break apart section by section;
  Newtonian flight with a diegetic autopilot (GOTO, ORBIT, STOP); a spinal
  railgun that rakes a corridor through a hull; a mod format where every ship
  and scenario is plain RON; a comic that shares the campaign's story.
- Three feature bullets, each provable with a screenshot or a loop.
- The comparable games a Steam tag search should land beside, and the tags.
- What the store page must NOT promise: open world, stations, the grown ship
  cast, the mobile pad, or full gamepad support. These are backlog work, not
  launch features. The page may describe the current build as an initial demo.
- State the release model exactly: free itch.io demo, public source on GitHub,
  Steam Coming Soon page, and no announced price for a future Steam build.

## Assets

Everything comes from the capture pipeline, not from hand screenshots, so a
re-shoot after a fix is one command. Reuse the `screenshots/` loops and the
release news captures.

- A 30-60 s gameplay-first trailer: cut from the existing loops (hull
  collapse, railgun, torpedo bay iris, GOTO arrival, the editor) plus two new
  beats the pitch needs. Show recognizable play in the first 10 seconds and
  make the hook readable without audio. Export H.264/AAC stereo MP4 at
  1920x1080, 30 or 60 fps, and at least 5 Mbps.
- Eight to ten real gameplay stills at 1920x1080 and 16:9. Keep the HUD where
  it explains play, omit marketing text and concept art, and remove the debug
  status bar (`45372be15` precedent). Steam requires at least five; identify
  at least four non-violent stills that can be marked suitable for all ages.
- Steam store capsules: header 920x430, small 462x174, main 1232x706, and
  vertical 748x896.
- Steam library art: capsule 600x900, header 920x430, hero 3840x1240 without
  a logo, and a transparent logo up to 1280 wide or 720 tall.
- An itch.io cover at 630x500, using its 315:250 aspect ratio, plus a page
  color and typography treatment that remains legible on desktop and mobile.
- Shared store copy: one-line hook, Steam short description, long description,
  three proved feature bullets, controls, current content, public-source link,
  developer/publisher names, supported languages, and Windows/macOS/Linux
  requirements derived from the release builds.
- Platform metadata: Steam genres, tags, supported features, mature-content
  answers, and release-date wording; itch.io classification, development
  status, genre, tags, accessibility, and download labels.

The Steam dashboard is authoritative if its live checklist requests an asset
not listed here. Record the added requirement before producing it.

## Feedback loop

Show the pitch and the trailer cut to three people who have not seen the
game. Record what they think the game is after 30 seconds. Revise until the
answer matches the hook.

## Done when

`PITCH.md` is approved by the owner; every source and exported asset above is
committed under this task; the copy does not promise backlog features; and the
Steam and itch.io tasks can populate their pages without making new marketing
material.

Requirements checked 2026-09-18 against Steamworks `Coming Soon`, `Graphical
Assets - Overview`, `Library Assets`, `Trailers`, and `Review Process`, plus
itch.io `Your first itch.io page`, `Pricing`, and `Uploading HTML5 games`.
