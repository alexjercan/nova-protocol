# Steam Coming Soon page with a wishlist button

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.14.0, release, distribution, steam

## Goal

A public Steam store page in Coming Soon state, so players can wishlist the
game before any build ships there. No demo, no release build on Steam in this
cycle. Owner (2026-09-09): "at least a landing page with wishlist capabilities
on Steam, we don't have to release a demo yet or anything."

Owner decision, 2026-09-18: publish this page after the v0.14.0 GitHub release.
Prepare the account and page earlier if useful, but do not make it public before
the release. A future Steam build and its price remain undecided.

## Steps

Requirements were checked against current Steamworks documentation on
2026-09-18. Recheck the live product checklist before submission because it is
the final authority.

### Owner account work

1. Complete Steamworks onboarding as an individual or legal entity, including
   identity, bank, and tax information.
2. Buy one Steam Direct app credit. Valve currently charges 100 USD plus
   applicable tax; it is recouped only after the product reaches 1,000 USD in
   adjusted Steam gross revenue. This fee applies even while only a Coming Soon
   page is public.
3. Create the app and record its app ID, owner account, public URL, developer
   name, and publisher name on this task.

### Populate the store page

1. Import the approved task `20260909-212954` outputs:
   - store header 920x430, small capsule 462x174, main capsule 1232x706, and
     vertical capsule 748x896;
   - library capsule 600x900, library header 920x430, hero 3840x1240, and
     transparent library logo;
   - at least five 1920x1080, 16:9 gameplay screenshots, with at least four
     suitable-for-all-ages images marked where their content permits;
   - the gameplay-first 1920x1080 MP4 uploaded to Steam, not linked from
     another video host;
   - approved short and long descriptions.
2. Fill the creator, supported language, Windows/macOS/Linux requirements,
   genre, tags, supported features, mature-content survey, legal, and release
   date fields. List only features that will exist when a Steam build launches.
   Do not claim open world, stations, expanded ships, mobile controls, or full
   gamepad support.
3. Use Coming Soon release-date wording. Do not set a product price, upload a
   build, create depots, or advertise a Steam demo in this task.

### Review and publish

1. Complete every item under Steamworks `Your Store Presence` and inspect the
   preview at desktop and narrow widths.
2. Mark the page ready for review at least seven business days before the
   intended publication date. Valve says store review typically takes 3-5
   business days. Record every rejection and its resolution.
3. After approval and after v0.14.0 is released, select `Post as Coming Soon`.
   Verify the page publicly and add it to a logged-in account's wishlist.
4. Add `Wishlist on Steam` beside `Play in browser` in
   `web/src/index.html` `.hero__cta`. Add a Steam widget to the v0.14.0 News
   post only if it improves the page rather than duplicating that action.
5. Record the publication time, public URL, app ID, review outcome, and proof of
   the working website link and wishlist action.

## Not in scope

- Steam builds, depots, Steam Input, achievements, cloud saves, and Steam
  product pricing. No Steam build ships in this cycle.
- The trailer cut, screenshots, and copy are produced by the store presence
  task; this task consumes them.
- The free downloadable demo is published on itch.io by `20260824-130004`.

## Done when

The Coming Soon page is live, the wishlist button works from a logged-in
account, and the landing page links to it.
