# v0.14.0 store sprint TODO

Research and repository audit: 2026-09-21. Steamworks' live product
checklist is authoritative when it differs from this file.

## End state

- The Steam Coming Soon page is public and accepts wishlists.
- The page promises only the game that exists. No Steam build, depot, demo,
  price, Early Access claim, or release date promise ships in this sprint.
- The page sends players to the existing free itch.io demo through a placement
  allowed by Steam.
- Steam, the project website, and GitHub form one verified path. The existing
  itch.io demo may link to Steam, and the project website links itch.io from
  the landing hero and the landing resource directory.
- Tasks `20260909-212954`, `20260909-212935`, and this epic can close.

Anything incomplete, misleading, at the wrong size, or rejected by the live
Steamworks checklist must block publication. Do not publish around a failed
check.

## Sprint snapshot

Already done:

- [x] v0.14.0 is tagged and live on GitHub.
- [x] The project website and browser build are live.
- [x] The itch.io page and v0.14.0 browser, Windows, macOS, and Linux channels
  are live at <https://alexjercan.itch.io/nova-protocol>.
- [x] The v0.14.0 news stills and 12 WebM capture loops exist.
- [x] The hook and five feature sections exist in
  `../20260909-212954/PITCH.md`.

Open sprint tasks:

- [ ] `20260909-212954`: finish and validate store copy, trailer, screenshots,
  capsules, library art, icons, metadata, and the three-person comprehension
  check.
- [ ] `20260909-212935`: onboard, create the app, populate, submit, publish,
  and verify the Steam Coming Soon page.
- [ ] `20260909-214736`: close only after the public page and cross-links work.

Owner updates after the audit:

- [x] Keep itch.io `Name your own price`. The owner accepts the current live
  setting; changing it to `No payments` is no longer sprint work.
- [x] 2026-09-21, superseding the earlier no-itch-CTA note: the project
  website carries platform links. The landing hero and the landing resource
  directory each show GitHub, the live itch.io page, and a static
  `Steam page pending` item with no href. The shared header carries Home, Wiki,
  Create, News, and Play. Create branches to the creator docs and the developer
  book; Wiki branches to Story and the lore encyclopedia. GitHub branches from
  the landing resource directory. The shared footer has no links. The existing
  browser and native-download actions remain the primary play paths.

Remaining live integration work:

- [ ] Add the Steam link to itch.io after the Steam URL exists.
- [ ] Add the Steam wishlist action to the project website after the Steam URL
  exists, and turn the landing page's static `Steam page pending` items into
  real links at the same time.
- [ ] Record clean-platform launch proof or link to where it was recorded. The
  closed itch.io task states the required result but does not contain the
  machine, OS, checksum, and observed-result records it requested.

## Critical path

### 1. Resolve owner-only Steam decisions

- [ ] Choose the Steamworks contracting party: individual/sole proprietor or
  legal entity. The legal name must match the bank and tax records.
- [ ] Choose the public developer and publisher names.
- [ ] Choose an honest internal intended release date. Steam requires an exact
  backend date even when the public display is `Coming Soon`.
- [ ] Confirm the public date display is `Coming Soon`. This puts the game
  behind titles with specific dates in upcoming lists, which is acceptable for
  an unannounced release date.
- [ ] Confirm that the future Steam product is represented by the current pitch.
  Valve reviews the page against what will be available when that product
  launches, not only against the itch.io demo.
- [ ] Decide whether to commission the capsule key art. Recommendation: hire an
  artist with game capsule experience if the current title treatment and key
  art do not remain clear at the small-capsule size.

### 2. Complete Steamworks onboarding and create the app

- [ ] Sign the NDA and Steam Distribution Agreement.
- [ ] Complete identity verification and bank and tax forms. Watch for requests
  for extra documents; Valve says tax verification can take 2-7 business days.
- [ ] Pay and activate one Steam Direct app credit: USD 100 or local equivalent,
  plus applicable tax. Steam Wallet funds cannot pay it. The fee is not
  refundable and is recouped only after USD 1,000 adjusted gross revenue.
- [ ] Record the fee date. Valve enforces a 30-day wait between paying the app
  fee and releasing the product. This does not prevent preparing or publishing
  its Coming Soon page.
- [ ] Create the app.
- [ ] Record the app ID, owner account, developer name, publisher name, and
  intended public URL in `../20260909-212935/TASK.md`.
- [ ] Open `Your Store Presence` and copy every live required field and asset
  into the delivery checklist before production. Add any requirement missing
  below.

### 3. Finish the store material

#### Copy and positioning

- [ ] Expand `../20260909-212954/PITCH.md` into the approved source for:
  - one-line hook;
  - Steam short description;
  - detailed `About This Game` copy;
  - Build, Fly, Combat, Damage, and Cockpit feature sections;
  - current controls and content scope;
  - supported languages;
  - Windows, macOS, and Linux requirements based on v0.14.0;
  - developer and publisher names;
  - genre, supported-feature, content-survey, and release-date answers.
- [ ] Keep the current hook: `Build a modular ship, fly it with real physics,
  and watch every hit tear it apart.` Test it before rewriting it.
- [ ] Describe actual play before implementation details. Lead with modular ship
  construction, Newtonian flight, and visible sectional destruction.
- [ ] Do not promise open world, stations, the expanded ship cast, mobile
  controls, full gamepad support, or a Steam demo.
- [ ] Do not imply whether the future Steam product is free or paid.
- [ ] Do not place itch.io, GitHub, or website links inside the description.
  Valve's review guidance says the description must not contain links to other
  sites. Use only dashboard-approved external-link fields. If no approved field
  can link itch.io directly, link the official website and make its itch.io
  action obvious; record this exception against the epic's direct-link wording.
- [ ] Proofread for detailed, coherent English and exact feature claims.

#### Tags and market position

- [ ] In the Tag Wizard, compare the game against actual neighboring audiences:
  Cosmoteer, NEBULOUS: Fleet Command, Children of a Dead Earth, Avorion, Space
  Engineers, and From the Depths. These are research candidates, not claims
  that Nova has all of their features.
- [ ] Select at least 5 accurate tags and build a relevant set of up to 20.
  Steam uses the top 20 for visibility and some filters prioritize the first 15.
- [ ] Make the top 5 explain the game without generic filler. Validate exact
  available names in the wizard. Candidate concepts are Space, simulation or
  space simulation, physics, building/customization, and destruction/combat.
- [ ] Exclude Open World, Survival, Roguelike, Multiplayer, Early Access, and
  other tags that the current product does not support.
- [ ] Check the wizard's similarly tagged titles. Reorder or remove tags if the
  results describe a different game.

#### Trailer

- [ ] Cut one 30-60 second gameplay-first trailer from the v0.14.0 capture loops
  plus any missing beats.
- [ ] Show recognizable player interaction in the first 5 seconds. Do not open
  with studio, engine, or title logos.
- [ ] Make the Build -> Fly -> Fight -> Break loop understandable in under 10
  seconds and without audio. Valve warns that Discovery Queue viewers may give
  the trailer less than 10 seconds and may watch muted.
- [ ] Keep real HUD elements where they explain play. Valve explicitly says HUD
  can help viewers identify what they will do.
- [ ] Use only shipped or committed in-engine behavior. No fake future feature
  footage.
- [ ] Export 1920x1080, 16:9, H.264 video, AAC stereo, MP4, 30 or 60 fps, and at
  least 5,000 Kbps. Use 44 kHz or 48 kHz audio.
- [ ] Inspect the encoded file for text legibility, pacing, audio levels, frame
  errors, and the first-frame/poster image.
- [ ] Upload it as the first trailer and categorize it as Gameplay. Steam derives
  its six-second microtrailer from the first listed video, so the full cut must
  contain useful images throughout.
- [ ] Wait for transcoding and inspect the Steam result. A stuck or failed
  transcode blocks submission.

#### Screenshots

- [ ] Select 8-10 real 1920x1080 16:9 gameplay stills. Steam requires at least 5.
- [ ] Make the first four tell a clear sequence: build, fly, fight, break.
- [ ] Include the editor, Newtonian flight/docking, cockpit HUD, torpedo or PDC
  combat, railgun impact, and visible sectional damage without repeating the
  same composition.
- [ ] Remove debug bars, capture controls, marketing copy, awards, and prose.
  Do not use concept art or pre-rendered stills.
- [ ] Mark at least four non-violent and non-suggestive screenshots as suitable
  for all ages. Without enough marked images, Steam may omit the game from some
  hover and front-page placements.
- [ ] Inspect every thumbnail at store size as well as full size.

#### Graphical assets

- [ ] Create and validate the current required store assets:
  - header capsule: 920x430;
  - small capsule: 462x174;
  - main capsule: 1232x706;
  - vertical capsule: 748x896;
  - shortcut icon: 256x256 PNG or ICO;
  - app icon: 184x184 JPG.
- [ ] Create and validate the current required library assets:
  - library capsule: 600x900;
  - library header: 920x430;
  - library hero: 3840x1240 PNG, with no text;
  - transparent library logo: 1280 wide and/or 720 tall.
- [ ] Use only the readable game name/logo and artwork on base capsules. Do not
  add taglines, review text, awards, discount text, or other marketing copy.
- [ ] Make the small capsule logo nearly fill its available area and test it at
  actual size. Steam review rejects an unreadable title.
- [ ] Keep critical hero artwork inside Steam's 860x380 center safe area.
- [ ] Download and use Valve's current templates. Steam no longer accepts the
  old pre-August-2024 capsule dimensions.
- [ ] Commit source files and exports under `../20260909-212954/` with an asset
  manifest that records dimensions, format, source capture, and license/owner.

#### Comprehension check

- [ ] Show the capsule, first 10 trailer seconds, and top four screenshots to
  three people who have not seen Nova.
- [ ] Ask each person what genre it is, what the player does, and what is unique.
  Do not explain the game before they answer.
- [ ] Record the answers. Revise if they do not identify modular ship building,
  physics-based space flight/combat, and sectional destruction.
- [ ] Change one major variable at a time so later Steam metrics can identify
  whether the capsule, trailer, copy, or tags helped.

### 4. Populate and review the Steam page

- [ ] Fill every `Your Store Presence` field, including basic info, creator
  names, languages, platform requirements, genres, supported features, tags,
  mature-content survey, legal fields, external links, and release date.
- [ ] Select only launch-real supported features. Do not select controller,
  multiplayer, achievements, cloud, workshop, or other features on intent.
- [ ] Upload the final assets and publish all pending store-page changes to the
  private preview.
- [ ] Inspect the preview at desktop and narrow widths. Check title legibility,
  media order, cropping, spelling, feature scope, and muted trailer behavior.
- [ ] Run a second claim audit against v0.14.0 and the planned Steam product.
  Remove incomplete features from screenshots, trailer, tags, and copy.
- [ ] Submit with `Mark As Ready For Review` at least 7 business days before the
  desired page publication. Valve says review usually takes 3-5 business days.
- [ ] Record each Valve response and resolution in
  `../20260909-212935/TASK.md`. Resubmit after any rejection.
- [ ] After approval, click `Post as Coming Soon`. Approval alone does not make
  the page public.

### 5. Treat page publication as an announcement

Steam does not promise a visibility boost merely because a Coming Soon page was
published. The announcement must bring its own traffic.

- [ ] Pick one publication day after review approval. Prepare all links and
  posts before clicking `Post as Coming Soon`.
- [ ] Publish one gameplay-first announcement trailer and one clear CTA:
  `Wishlist Nova Protocol on Steam`.
- [ ] Announce through channels already owned by the project: website, itch.io,
  GitHub release/community surfaces, and any existing mailing list or social
  accounts. Do not create a channel with no plan to maintain it.
- [ ] Contact a small, specific set of space-sim, ship-building, and physics-game
  creators or communities. Follow each community's promotion rules. Do not
  mass-post generic copy.
- [ ] Use distinct Steam UTM links for the website, itch.io, GitHub, each social
  channel, creator outreach, and any press. Test each link in Steamworks.
- [ ] Record a baseline at publication, then daily additions for the first 14
  days. Also record trusted/tracked visits, UTM-attributed wishlists, deletes,
  and page impressions where Steam exposes them.
- [ ] Do not turn third-party wishlist benchmarks into a sprint pass/fail goal.
  Compare Nova's channels and matched periods against its own baseline.

### 6. Public verification and closure

- [ ] Open the public page signed out and signed in on desktop and a narrow
  viewport.
- [ ] Add it to a logged-in account's wishlist, reload, and verify the state.
- [ ] Verify the public trailer, first four screenshots, capsules, tags, creator
  names, supported platforms, date wording, and external link route.
- [ ] Add `Wishlist on Steam` beside `Play in browser` in
  `web/src/index.html`, using a tested UTM link.
- [ ] Add the Steam URL to itch.io. Keep the accepted `Name your own price`
  setting.
- [ ] Verify website -> Steam and itch.io -> Steam. If Steam exposes an approved
  external-link field for itch.io, also verify Steam -> itch.io; do not place
  the link inside `About This Game`.
- [ ] Record publication time, app ID, public URL, review outcome, UTM scheme,
  and verification evidence in `../20260909-212935/TASK.md`.
- [ ] Close `20260909-212954` only when every source and export is committed and
  the three-person check is recorded.
- [ ] Close `20260909-212935` only when the page is public and a real wishlist
  action works.
- [ ] Close this epic only after the required Steam and website links are public.

## Research conclusions

Authoritative Steamworks findings:

- A Coming Soon page needs branding, copy, a completed Store Presence checklist,
  Valve review, and a separate `Post as Coming Soon` action. A build is not
  required for this sprint's normal non-adult store page.
- Submit at least 7 business days before the intended page date; normal store
  review is 3-5 business days.
- A new product must remain Coming Soon for at least two weeks before product
  release. Steam Direct also has a separate 30-day fee-to-release wait.
- Steam recommends publishing when the genre, art direction, screenshots, and
  description are ready. There is no strong stated penalty for a long Coming
  Soon period if the game does not change enough to confuse early wishlisters.
- Trailer viewers may be muted and may give the game less than 10 seconds.
  Gameplay should be first and should show the player's perspective.
- At least five gameplay screenshots are required. At least four suitable images
  should be marked suitable for all ages for broader placement eligibility.
- Tags affect browse and recommendations. At least five are required before
  launch; up to twenty are recommended, and order matters.
- There is no minimum wishlist count at which Steam starts showing a game.
  Wishlists are an outcome of awareness work, not a substitute for it.
- Steam UTM analytics can attribute a wishlist within 72 hours of a tracked
  visit. Visit data updates hourly; conversions finalize after four days.

Advisory marketing findings from How To Market A Game:

- Publish once genre and visual style are stable, there is enough visual variety
  to prove depth, the capsule is credible, and a short gameplay trailer exists.
- Do not wait indefinitely for ideal assets or a large announcement partner.
  A first-time developer gains more from having a usable wishlist destination
  than from months of silence.
- A Steam page launch has no automatic algorithmic boost. Coordinate it as an
  announcement and bring an audience to it.
- Capsule art is the top of the store funnel. It must identify the genre before
  copy explains Nova's differences.
- Test the message before launch. If viewers cannot name the genre and player
  activity, more promotion will amplify a weak page.
- Measure impressions -> visits -> wishlists and improve against Nova's own
  baseline. Third-party conversion and wishlist-count anecdotes are context,
  not guarantees.
- Accurate, specific top tags are better than broad tags such as Indie, Action,
  or Adventure. Current Steamworks guidance still recommends a complete set of
  up to 20, so use specific tags first rather than stopping at 5 or 6.
- The itch.io demo is useful as a proof and feedback path. It can point
  interested players toward the Steam wishlist without implying the future
  Steam price or scope. The project website links it as one platform among
  GitHub and the pending Steam page, below the browser and download actions.

## Sources

Official, authoritative:

- [Steamworks: Coming Soon](https://partner.steamgames.com/doc/store/coming_soon)
- [Steamworks: Onboarding](https://partner.steamgames.com/doc/gettingstarted/onboarding)
- [Steamworks: Steam Direct Fee](https://partner.steamgames.com/doc/gettingstarted/appfee)
- [Steamworks: Review Process](https://partner.steamgames.com/doc/store/review_process)
- [Steamworks: Graphical Assets Overview](https://partner.steamgames.com/doc/store/assets)
- [Steamworks: Store Graphical Assets](https://partner.steamgames.com/doc/store/assets/standard)
- [Steamworks: Graphical Asset Rules](https://partner.steamgames.com/doc/store/assets/rules)
- [Steamworks: Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)
- [Steamworks: Trailers](https://partner.steamgames.com/doc/store/trailer)
- [Steamworks: Tags](https://partner.steamgames.com/doc/store/tags)
- [Steamworks: Release Dates](https://partner.steamgames.com/doc/store/release_dates)
- [Steamworks: Wishlists](https://partner.steamgames.com/doc/marketing/wishlist)
- [Steamworks: UTM Analytics](https://partner.steamgames.com/doc/marketing/utm_analytics)

Advisory, not Valve policy:

- [Launching a Steam Coming Soon page the right way](https://howtomarketagame.com/2023/06/06/launching-a-steam-coming-soon-page-the-right-way/)
- [When should I post my Steam Coming Soon page?](https://howtomarketagame.com/2025/03/10/when-should-i-post-my-steam-coming-soon-page/)
- [Why your Steam page matters](https://howtomarketagame.com/2020/11/23/why-your-steam-page-matters/)
- [Steam 101: how to tag your game](https://howtomarketagame.com/2020/11/12/steam-101-how-to-tag-your-game/)
- [Five trends in Steam capsule art](https://howtomarketagame.com/2020/07/27/five-trends-in-steam-capsule-art/)
- [What is wishlist velocity?](https://howtomarketagame.com/2024/06/04/what-is-wishlist-velocity-and-is-it-a-better-indicator-of-success/)
- [Games that moved from itch.io to Steam](https://howtomarketagame.com/2025/05/22/more-games-that-made-the-itch-io-to-steam-transition/)
