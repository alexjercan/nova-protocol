# Retro: Implement ammo limit logic

- TASK: 20260525-133025
- BRANCH: v0.5.0/combat-depth (squash-landed to master as e1b949f)
- REVIEW ROUNDS: 0 (autonomous flow, self-review only - no separate /review agent)

## What went well

- Building `SectionAmmo` as opt-in (absent component = unlimited) meant zero
  churn to the ~47 existing turret/torpedo firing tests - they never asked for
  ammo, so they never got gated. The backward-compatible default paid for itself
  immediately.
- The exact-count assertion is robust by construction: `try_consume` hard-caps
  total consumption at the magazine size, so "fires exactly k bullets, ever" holds
  regardless of the sub-tick fire timing that would have made a
  bullets-per-tick assertion rig-fragile. Picking the invariant that does not
  depend on the messy part of the system made the test trustworthy.
- The no-ammo A/B control (same rig, no `SectionAmmo`, fires past k) proves the
  gate - not some other limit - stopped the stream.

## What went wrong

- The torpedo ammo test first read 1 launch instead of the expected magazine,
  then (once fixed) the unlimited control read 2 instead of "many". Root cause:
  the test rig was not production-faithful. First miss: the bay's fire timer is
  ticked by a SEPARATE system (`update_spawner_fire_state`), unlike the turret
  which ticks inside its own fire system - my rig ran only `shoot_spawn_projectile`,
  so the timer never re-armed. Second miss: `Time<Virtual>`'s 0.25s max-delta
  clamp starved the bay's 1s fire interval, so a 2s manual dt only advanced 0.25s
  and the timer needed four ticks to re-arm. Both were mechanism facts I had to
  trace from the failing count, not guess.
- Shipping finite ammo with no HUD readout and no reload (reload is the next
  pass) is a half-feature from the player's chair. Caught it at self-review and
  filed task 20260712-131348 rather than letting it reach playtest as a silent
  "why can't I shoot".

## What to improve next time

- Before writing a headless firing/timer test, list which production systems tick
  the state under test and add ALL of them to the rig - and sanity-check the
  manual dt against `Time<Virtual>`'s max-delta clamp, or the timer silently
  under-advances. (production-faithful-rigs, again.)
- When a feature has an obvious player-feedback half (a counter, a HUD, an audio
  cue), decide its fate at plan time - ship it, or file it - so it is a decision,
  not an omission.

## Action items

- [x] tatr 20260712-131348: ammo HUD readout (filed, follow-up)
- [x] LESSONS.md: bumped production-faithful-rigs and tatr-same-second-collision
