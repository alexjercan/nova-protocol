# Spike: Roadmap reprioritization - what to drop, what to commit to v0.5.0, and what new "feel" features to add

- DATE: 20260708-203517
- STATUS: RECOMMENDED
- TAGS: spike, roadmap, planning, juice

## Question

v0.4.0 ("sections polish + testability") is shipped. The remaining backlog is
29 OPEN tasks, all tagged v0.5.0 (23) or v0.6.0 (6), and it was accreted across
several earlier planning passes without a single owning direction. Three
decisions need making, together, before the next release is planned:

1. Which backlog items should be **dropped** (priority 0, pushed to a later
   tag) because they do not serve where the game is going?
2. Which items should be **committed to the next release and prioritized**?
3. What **new features** does the chosen direction imply that the backlog does
   not yet contain?

A good answer is a concrete, faithful reprioritization of the existing tasks
plus a short list of seeded new tasks, all anchored to an explicit statement of
where the game is headed.

## Context

Nova Protocol is a 3D space game on Bevy 0.19 + avian3d (see
`docs/architecture.md`). The heavy engineering investment to date is telling:
a section-by-section ship **integrity/destruction** system, a **PDC turret**
with lead/intercept aiming, and a **torpedo** with proportional-navigation
guidance and blast damage. That is the vocabulary of hard-sci-fi naval combat,
not arcade dogfighting.

Two prior spikes already set direction for large chunks of the backlog and
should not be relitigated here:

- `tasks/20260708-161726/SPIKE.md` - modding is
  staged: a declarative **RON scenario format** first, then a **piccolo**
  scripting VM much later, prototype-gated.
- `tasks/20260708-165647/SPIKE.md` - the weapons HUD is one
  **shared screen-projection substrate** with cheap-first consumers (lead pip,
  target readout) before the mechanics-bearing ones (dwell cue, edge arrows).

### Direction decided (this spike)

The user set the direction explicitly:

- **Identity:** a *shippable game* (content and feel win over library polish).
- **Fantasy:** *hard-sci-fi capital combat* - Expanse-style. Heavy ship, PDC
  screens intercepting incoming torpedoes, section-by-section destruction,
  positioning and burns over twitch. This validates the existing
  integrity/PDC/PN-guidance investment as the core, not a side quest.
- **Next release (v0.5.0) theme:** *Combat feel & juice.* Make the combat that
  already exists feel good before adding more content or mechanics.
- **Juice priorities (all four selected):** spaceship handling, sound design,
  hit/destruction FX, and HUD readability.

The single most important gap this surfaced: **there is no task for spaceship
handling at all**, yet the user ranked it a top-tier juice dimension and it is
foundational to the capital-combat fantasy (a heavy ship has to *feel* heavy).

## Options considered

### How to slice the releases

- **A. One big "make it a game" release** - juice + objectives + modding
  format all in v0.5.0. Rejected: that is the same over-scoping that made
  v0.4.0 need re-narrowing; it also contradicts the user's explicit choice of
  *feel* over *content* for the next release.
- **B. Feel-only v0.5.0, then content, then platform (recommended).** v0.5.0 is
  a tight "combat feel & juice" release; the objectives/win-lose/scenario-format
  content epic becomes v0.6.0; modding scripting, weapon variety, editor
  save/load, and the docs pass become v0.7.0. Each release has one owning theme
  and a clear "is it done" line. Matches the user's ranking exactly.
- **C. Keep the flat v0.5.0/v0.6.0 split as-is.** Rejected: the current split
  mixes headline feel work (audio p55, hit-FX p50) *below* content work
  (scenario format p80, config resource p75) inside the same tag, so priority
  order fights the chosen theme.

### What counts as "drop" here

Nothing is deleted. "Drop" = priority 0 (or near-0) and retagged to a **later**
release, so it stops competing for attention now without losing the idea.
Candidates are the items that serve identities the user *did not* pick
(reusable-library polish, premature optimization, low-end-machine support) or
that are strictly downstream of the modding-platform phase.

## Recommendation

Adopt slicing **B** and reprioritize the backlog around a three-release
roadmap. Concrete per-task changes are in "Next steps".

### The roadmap

Bucketed into two tags per the user's call: **v0.4.0 is this sprint** (the
combat-feel/juice work, keeping its priorities); **v0.5.0 is everything else**,
parked at **priority 0** until the sprint lands. The v0.5.0 bucket still carries
its own internal ordering in this doc (content/missions before
platform/modding); those relative priorities are recorded here and can be
restored when v0.5.0 is planned - on the tasks themselves they are all 0 for now.

- **v0.4.0 (this sprint) - Combat feel & juice.** Headliners: audio/SFX, hit &
  destruction FX, a new spaceship-handling/flight-feel overhaul, smarter enemy
  AI, and HUD phase 1 (the projection substrate + turret lead pip +
  locked-target readout). A minimal faction/relation concept is pulled in
  because both the AI and the reticle work need it. Definition of done: the
  *existing* fight - fly the ship, screen incoming torpedoes with the PDC, kill
  a target and watch it come apart - feels good to play, with sound, weight, and
  legible HUD, even though there is no win/lose yet.
- **v0.5.0 (everything else, parked at p0) - two arcs, planned later.**
  - *Make it a game:* RON scenario format + config resource, hardcoded
    objectives + win/lose + objectives HUD, ammo limits, variable damage by
    section type, HUD phase 2/3 (lock-on dwell cue, off-screen threat arrows,
    multi-target cycle), and the playable capital-combat vertical slice.
  - *Content & modding platform:* piccolo scripting prototype, skybox action,
    editor polish + ship blueprint save/load, weapon/damage-type variety,
    section blueprints as data, and the documentation pass.
  When v0.5.0 is planned, re-split these two arcs (they were v0.6.0 and v0.7.0
  in the original three-release sketch below) and restore priorities.

### Why this beats the runners-up

It is the only slicing that honors the user's explicit ordering (feel before
content before platform) while giving each release a single theme and a
testable done-line. It leaves the two prior spikes' internal plans intact - it
only moves *when* those arcs land, not *what* they are.

### New features the direction implies (seeded below)

1. **Spaceship handling / Newtonian flight-feel overhaul** - the backlog gap.
   Flight-assist toggle (assisted vs full Newtonian), inertial damping, throttle
   control, and RCS/main-thruster feedback so a capital ship handles with
   weight. Top v0.5.0 priority alongside audio and hit-FX.
2. **Minimal faction/relation model** - hostile / neutral / your-own. Unblocks
   AI target selection (162012) and reticle-by-relation colouring (flagged as an
   open question in the weapons-HUD spike). Small, enabling, pulled into v0.5.0.
3. **Playable capital-combat vertical slice** - one real fight that dogfoods
   every juice system end to end (your ship vs an enemy that shoots torpedoes
   you must PDC-screen). The "shippable game" north-star demo; lands in v0.6.0
   once objectives give it a win/lose frame.

Two feel ideas are deliberately *folded into existing tasks* rather than seeded
as new ones, to avoid fragmentation: capital-ship destruction FX (hull sparks,
atmosphere venting, debris on section knock-off) folds into hit-feedback 162013;
thruster/RCS visual+audio feedback is shared between the new handling task and
audio 162011; combat camera shake is already inside 162013.

## Open questions

- **Sound in vacuum.** Hard-sci-fi realism says no sound in space; playability
  says cinematic SFX. Recommend cinematic-with-restraint (interior/hull-conducted
  framing), but this is a design call to settle when audio 162011 is planned.
- **Handling model default.** Assisted (auto-brake, snappy) vs full Newtonian as
  the *default*, with the other as a toggle. Leaning assisted-default for
  approachability; confirm when the handling task is planned.
- **Faction depth.** The minimal hostile/neutral/own triple is enough for v0.5.0;
  a fuller faction system (alliances, reputation) is its own future spike if the
  game ever needs it.
- **Does the player build ships?** The capital-combat fantasy does not require
  the editor to be player-facing, which is why editor save/load sits in v0.7.0.
  Revisit if a build-and-fight loop becomes a goal.

## Next steps

Reprioritization applied to existing tasks. NOTE ON TAGS: per the user's final
call the release tags were collapsed to two buckets - **v0.4.0** = this sprint
(the "Committed and raised" set below, priorities kept), and **v0.5.0** =
everything else (both the "Deferred" and "Dropped" sets below), all parked at
**priority 0**. The priorities shown in the Deferred/Dropped lists are the
*intended relative ordering* to restore when v0.5.0 is planned; on the tasks
today they are 0.

### Committed and raised - v0.4.0 "Combat feel & juice" (this sprint)

- 162011 audio/SFX system: p55 -> **p90** (headliner).
- 162013 hit feedback / juice: p50 -> **p88** (headliner; now also owns
  capital-ship destruction FX - sparks/venting/debris).
- 162012 smarter enemy AI: p60 -> **p80**.
- 165700 HUD projection substrate: p40 -> **p78**.
- 165701 turret lead/intercept pip: p38 -> **p72**.
- 165702 locked-target info readout: p36 -> **p70**.
- 133024 torpedo bay particles: p30 -> **p48** (native works; wasm blocked on
  162908).
- 162908 re-enable particles on wasm: stays v0.5.0 **p10** (blocked externally).

### Deferred to v0.5.0 "Make it a game" arc (retagged v0.5.0, now p0; intended order shown)

- 133029 scenario language/config format (RON) -> v0.6.0 p80.
- 133028 scenario config resource -> v0.6.0 p75.
- 133026 scenario objectives + win/lose -> v0.6.0 p70.
- 133027 objectives HUD -> v0.6.0 p65.
- 133025 ammo limit logic -> v0.6.0 p45.
- 133004 variable damage by section type -> v0.6.0 p45.
- 165703 lock-on dwell + acquire/lock cue -> v0.6.0 p50 (pairs with audio).
- 165704 off-screen target/threat edge indicators -> v0.6.0 p40.
- 165705 multi-target tracking + subtarget cycle -> v0.6.0 p35.
- 133012 verify post-processing camera marker wired -> v0.6.0 p20.

### Dropped to v0.5.0 "Content & modding platform" arc (retagged v0.5.0, now p0; intended order shown)

- 162010 piccolo scripting VM prototype -> v0.7.0 p30 (platform, spike-gated).
- 162014 editor polish + ship blueprint save/load -> v0.7.0 p25.
- 162005 weapon & damage-type variety (alt-fire, AP/EMP) -> v0.7.0 p20.
- 133017 skybox cubemap via action -> v0.7.0 p20.
- 133014 optimize modding event handler lookup -> v0.7.0 p0 (premature).
- 133013 spawn-less visual mode for low-end machines -> v0.7.0 p0.
- 133010 minimal example for bevy_common_systems -> v0.7.0 p0 (other repo).
- 133030 nova_gameplay API docs -> v0.7.0 p15.
- 133031 bevy_common_systems API docs -> v0.7.0 p0 (other repo).
- 133032 inline doc comments on public items -> v0.7.0 p15.
- 133033 general documentation pass -> v0.7.0 p15.

### New tasks seeded (this spike)

- New (v0.5.0, handling): Spaceship handling / Newtonian flight-feel overhaul.
- New (v0.5.0, factions): Minimal faction/relation model (hostile/neutral/own).
- New (v0.6.0, example): Playable capital-combat vertical-slice scenario.
