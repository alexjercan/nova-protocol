# Epic: v0.13.0 puts the game on screen

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.13.0,epic

Planned 2026-08-31 with the owner, the round after the v0.12.0 release.
The v0.12.0 epic (`20260812-131912`) is the precedent for shape.

## The release arc

Three steps, decided together:

- **v0.13.0 - this release.** The concrete features and polish from the
  backlog: the content and feel round.
- **v0.14.0 - the release release.** Bugfixes, balancing and final polish
  only, then packaged releases on itch.io and Steam (`20260824-130004`).
  No new features. Its board is planned when v0.13.0 closes.
- **After v0.14.0 - the promises.** Open world (`20260824-125938`), space
  stations (`20260824-125943`), a grown ship cast (`20260824-125951`), the
  PDC stow (`20260831-083622`), the agent that plays the game
  (`20260824-125933`), the mobile virtual pad (`20260831-145917`). They
  stay in the backlog at priority 0, advertised as future work, not
  scheduled.

## The release story

v0.12.0 made the editor the star. v0.13.0 puts the game itself on screen:
every section a real model, a new spinal weapon, more campaign to play,
the first dedicated audio pass, a gamepad actually in hand, and a console
that reaches into the world by name.

## The board

- p80 `20260831-083625` - section models to the thruster's standard
- p72 `20260824-125947` - railgun: a spinal kinetic weapon family
- p68 `20260824-125959` - more campaign chapters after the ledger
- p60 `20260824-125955` - the audio direction pass
- p55 `20260714-001140` - gamepad navigation, and a playthrough on hardware
- p50 `20260827-120347` - the console and the action vocabulary
- p45 `20260824-160705` - autopilot pacing and probe contracts
- p10 `20260831-145920` - release v0.13.0

Ordering: models first, because the railgun wants the section quality bar
and the bay's section-animation decision, and the campaign wants both on
screen. Audio after there is content to score. The console and the
autopilot internals run independent of the spine.

## Release definition of done

- Every v0.13.0 task is closed or explicitly cut with the cut recorded on
  the task. A concern that surfaces as bugfix or balance rather than
  feature moves to the v0.14.0 board instead of growing this release.
- Full correctness probe, content lint, Rust checks, and web CI green on
  master.
- Documentation ships with the behavior it describes.
