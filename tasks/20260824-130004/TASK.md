# Ship v0.14.0 to itch.io and Steam

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.14.0,release,distribution

Promoted 2026-08-31 from ideation: v0.14.0 is the release version. The
cycle is bugfixes, balancing and final polish - no new features - and it
ends with the game on the stores. Distribution beyond the web build: the
game reaches players who never open a repo.

## Scope

- Packaged native releases on itch.io AND Steam (owner 2026-08-31; Steam
  moved from "maybe later" into the goal). The nix package
  (`20260824-124117`, landed v0.12.0) is the reproducible base.
- Store presence is work of its own: page, screenshots, a trailer cut
  from the capture pipeline, the pricing/free decision, Steamworks setup
  and depots, itch.io butler uploads.
- The bugfix / balancing / polish board for v0.14.0 is planned when
  v0.13.0 closes; v0.13.0's deferrals are its raw material. A
  `Release v0.14.0` meta task runs last, per precedent.

## Done when

- The game is live on itch.io and Steam, installed from each store onto a
  clean machine, and launches to a playable state.

## Rescoped 2026-09-09 at v0.14.0 planning

Split. The store PAGE and the assets moved out so they run early:

- `20260909-212935` - the Steam Coming Soon page with the wishlist button.
- `20260909-212954` - the pitch, the trailer, the capsules, the copy.

This task keeps the SHIPPING half, and runs at the end of the cycle after
the release meta task tags the version:

- itch.io: the project page (from the store presence assets), butler
  uploads of the Windows, macOS, Linux, and web builds the release workflow
  already produces, the free-or-paid decision applied, the "Wishlist on
  Steam" link on the page.
- Steam: NOT a build in this cycle. The Coming Soon page stays a page. Record
  here what a first Steam build needs (Steamworks SDK or none, depots per
  OS, the launch options, Steam Input) so the next cycle starts from a list.
- The landing page's download buttons keep GitHub releases as the source;
  add the itch.io badge beside them.

Done when: the game is installed from itch.io onto a clean machine on each
OS and launches to a playable state, and the Steam page links to itch.io as
the place to play now.
