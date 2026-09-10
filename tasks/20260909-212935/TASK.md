# Steam Coming Soon page with a wishlist button

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.14.0, release, distribution, steam

## Goal

A public Steam store page in Coming Soon state, so players can wishlist the
game before any build ships there. No demo, no release build on Steam in this
cycle. Owner (2026-09-09): "at least a landing page with wishlist capabilities
on Steam, we don't have to release a demo yet or anything."

This runs EARLY in the cycle, not at the end: wishlists accrue while the
stabilization work lands, and Steam reviews a page before it goes live.

## Steps

Verify each requirement against the current Steamworks documentation before
acting; the figures below are from memory and may have moved.

1. Steamworks partner account: legal entity or individual, tax and bank
   interview, the per-app fee (100 USD, recoverable after 1000 USD net). Owner
   action; record the app id on this task.
2. Store page assets, from the store presence task's outputs:
   - capsule art in every required size (header, small, main, vertical,
     library hero and logo),
   - at least five screenshots at 1920x1080 or 16:9,
   - one trailer (a store trailer must be uploaded as a video, not a link),
   - short and long description, genre and feature tags, system requirements
     for Windows, macOS, and Linux, the age rating questionnaire.
3. Set the release date to "Coming soon" or a quarter. Enable the wishlist
   button by publishing the Coming Soon page after Steam's page review.
4. Landing page: add a "Wishlist on Steam" button in the hero beside "Play in
   browser" (`web/src/index.html`, `.hero__cta`). Add the Steam widget to the
   News post for the release, if the site uses one.
5. Record the page URL, the app id, and the review outcome on this task.

## Not in scope

- Steam builds, depots, Steam Input, achievements, cloud saves: the
  distribution task (`20260824-130004`) owns uploads when a build ships.
- The trailer cut, screenshots, and copy are produced by the store presence
  task; this task consumes them.

## Done when

The Coming Soon page is live, the wishlist button works from a logged-in
account, and the landing page links to it.
