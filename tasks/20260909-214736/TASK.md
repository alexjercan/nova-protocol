# Epic: v0.14.0 puts the game in the store

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.14.0, epic

Planned 2026-09-09 with the owner, the day the v0.13.0 tag was cut and
before its release meta task (`20260831-145920`) closed. The v0.13.0 epic
(`20260831-145934`) is the precedent for shape.

## The release story

v0.13.0 put the game on screen. v0.14.0 puts it in the store: the game on
itch.io, a Steam page taking wishlists, and the first chapter of the
season-one story playable. Everything else in the cycle is review, bugs,
balance and polish. Owner direction: "focus on more code review rather
than tasks in particular and also a lot of gameplay feedback"; "look into
hardcoded values"; "find potential bugs and edge cases, add more systems
examples tests"; "more splits to have easier CI"; "at least the first
scenario or so of the story"; "the release to itch.io and at least a
landing page with wishlist capabilities on Steam, we don't have to release
a demo yet"; "think on how to market the game and make it fun enough for
people to wishlist it."

No new engine features. A feature request that surfaces in this cycle goes
to the backlog at priority 0.

## How the board was built

Five read-only audits on 2026-09-09, each recorded in its task body:
three hardcoded-value sweeps (presentation, simulation, scenario and
editor) that found 61 sizes fixed in world units or pixels that should
derive from `HullRadius`, `BodyRadius`, the target's projected radius, or
a prototype's footprint; two bug hunts (simulation, lifecycle) that
confirmed 27 defects; a coverage map of `examples/systems/` against the
subsystems; and a mine of the v0.13.0 review records for 40 findings that
were reported and never fixed. CI timings were read from run 34317217860.

## The board

Review and bugs, first, in parallel lanes (one writer per sprout):

- p86 `20260909-212917` velocity and gravity spheres inscribe the hull
- p84 `20260909-213350` HUD, camera and map sizes derive from the hull
- p83 `20260909-213708` flight, AI, weapon and destruction figures derive
- p82 `20260909-213559` scenario, editor and authoring sizes derive
- p81 `20260909-214706` simulation defects
- p80 `20260909-214629` lifecycle defects
- p76 `20260909-213525` close the v0.13.0 review leftovers

Proof and CI:

- p72 `20260909-213441` systems ranges: session loop, settings, mods, picker
- p71 `20260909-213623` systems ranges: combat at both hull sizes
- p70 `20260909-213726` systems ranges: flight legs, gravity, AI patrol
- p66 `20260909-213100` split the probe matrix into balanced shards

Content and feel:

- p62 `20260908-161328` write the shared season-one story (the campaign
  brief is what chapter one needs)
- p58 `20260909-213032` chapter one: the Gantry rescue as a scenario
- p55 `20260909-213118` gameplay feedback ledger
- p50 `20260714-001140` gamepad navigation and the hardware playthrough

Store:

- p45 `20260909-212954` store presence: pitch, trailer, capsules, copy
- p40 `20260909-212935` Steam Coming Soon page with a wishlist button
- p30 `20260824-130004` ship to itch.io (Steam builds recorded, not shipped)
- p10 `20260909-213154` release v0.14.0

Ordering: the sphere fix and the three size sweeps first, because every
range written afterwards must run on the carrier as well as the skiff and
the fixes change what those ranges measure. The CI split lands before the
range tasks grow the suite. The store presence task starts as soon as the
pitch can be written; the Steam page follows it and runs long before the
release so wishlists accrue. Chapter one waits on the story brief, not on
the bug work.

## Backlog, deliberately not pulled

`20260831-145917` mobile virtual pad, `20260901-104359` railgun lance AI,
`20260824-125951` grown ship cast, `20260824-125943` stations,
`20260824-125938` open world, `20260907-173050` verified statistics wiki.
The first five are features; the wiki task is where balance figures would
publish and may be scheduled by the owner if the ledger produces them.

## Release definition of done

- Every v0.14.0 task closed or explicitly cut with the cut recorded on it.
- Full correctness probe green three runs in a row, content lint, Rust
  checks and web CI green on master.
- The feedback ledger has a disposition on every row.
- The game installs from itch.io on each OS and launches to a playable
  state; the Steam page is live and links there.
