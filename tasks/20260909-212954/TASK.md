# Store presence: the pitch, the trailer, the capsules, the copy

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.14.0, release, marketing, capture

## Goal

Make the game wishlist-able: decide the pitch, then produce the store assets
that the Steam page (`20260909-212935`) and the itch.io page
(`20260824-130004`) consume. Owner (2026-09-09): "think on how to market the
game and make it fun enough for people to wishlist it."

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
  cast, the mobile pad (backlog, "future work" at most).
- The free-or-paid decision, recorded with its reason.

## Assets

Everything comes from the capture pipeline, not from hand screenshots, so a
re-shoot after a fix is one command. Reuse the `screenshots/` loops and the
release news captures.

- A 30-60 s trailer: cut from the existing loops (hull collapse, railgun,
  torpedo bay iris, GOTO arrival, the editor) plus two new beats the pitch
  needs. Music and SFX from the shipped audio pass. Export at 1920x1080.
- Eight to ten stills at 1920x1080, HUD on for combat and off for beauty
  shots, no debug status bar (`45372be15` precedent).
- Capsule art in every Steam size and an itch.io cover, from the hero ship
  render, with the logo treatment the site already uses.
- Store copy: short description (under 300 characters), long description with
  the three bullets, the system requirements from the release builds.

## Feedback loop

Show the pitch and the trailer cut to three people who have not seen the
game. Record what they think the game is after 30 seconds. Revise until the
answer matches the hook.

## Done when

`PITCH.md` is approved by the owner, the trailer, stills, capsules and copy
are committed under this task, and both store tasks can proceed without
producing assets of their own.
