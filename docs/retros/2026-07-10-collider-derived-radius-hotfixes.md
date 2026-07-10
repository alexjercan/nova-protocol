# Retro: Direct-on-master stretch - gravity retune, collider-derived radius, band regression

- TASK: none (user-directed hotfixes on master, per request)
- COMMITS: 1866687 (gravity strength x2), a553729 (collider-derived
  BodyRadius + geometric orbit parking), 55ec492 (fix: well sized on the
  geometric radius - the "no stable band" regression fix)
- REVIEW ROUNDS: 0 - and that is the story of this retro

## What went well

- Root-causing "still stops too close" went to the actual data instead of
  another constant bump: reading bevy_common_systems' `apply_noise`
  (`pos + normalize(pos) * height`, height in [0, 5] unit-space) proved
  the real rock edge is up to several times the nominal radius. The fix
  derives the edge from the collider volume, so nothing about it can
  drift out of date again.
- The regression was diagnosable from a single user-pasted log line
  because every ORBIT disengage path logs a distinct reason ("has no
  stable band" pointed straight at orbit_target_radius). Distinct
  debug-reason strings on every refusal path keep paying off.
- `On<Add, BodyRadius>` as the well observer's trigger turned an observer
  ordering problem into a data dependency - the well physically cannot be
  built before the radius it needs exists. Sequencing through the data is
  sturdier than sequencing through registration order.

## What went wrong

- a553729 shipped a regression the USER found in playtest within minutes:
  O-key ORBIT refused with "no stable band" everywhere. Two root causes,
  both process:
  1. I changed ONE side of an inequality without recomputing the other
     with realistic magnitudes. The band is `1.5 * body_radius <=
     0.9 * 0.85 * soi_radius`; I raised the floor to the geometric radius
     (4-6x nominal) while the ceiling stayed nominal-derived, and never
     plugged in a realistic derived extent (~100u for a 20u rock) to see
     the band collapse. My own doc even celebrated the mismatch as
     "geometry and physics are deliberately separate readings".
  2. The direct-on-master workflow skipped the adversarial review that
     every task cycle gets - and this session's reviewers had caught
     exactly this class of cross-system interaction three times (eta
     fallback, handoff proximity gate, telemetry consumers). "On master
     directly" was the user's call for speed; skipping the numbers check
     was mine.
- Test coverage aimed where I was looking, not where the change reached:
  the new tests covered the arrival path (GOTO standoff, handoff ring)
  but no test exercised MANUAL orbit planning at an asteroid with a
  derived radius - flight tests use bare hand-built wells, scenario tests
  never ran the orbit planner. The gap between the two crates' fixtures
  is exactly where the regression lived.

## What to improve next time

- When retuning any guard or band, recompute BOTH sides with realistic
  values before shipping - a two-line arithmetic check in the commit
  message would have caught this ("floor 1.5*101 = 151 > ceiling 122").
- Direct-on-master does not mean review-free: for changes that couple
  systems (here: mesh generation -> well physics -> orbit planning), run
  the adversarial reviewer on the diff even without a task cycle, or at
  minimum walk every consumer of the changed value (the AGENTS.md
  derive-from-owning-system rule already demands this - it applies to
  hotfixes too).
- When a value's meaning changes (body_radius: nominal -> geometric),
  grep every reader and re-ask "which meaning does this site want?" -
  orbit_target_radius wanted the geometric one on BOTH terms, not one.

## Action items

- [x] AGENTS.md: added the recompute-both-sides line to the Conventions
  (this commit).
- [x] Playtest side effects flagged to the user: SOI now 8x the real
  radius (pull felt much farther out), park point at the visible edge.
- [ ] If the bigger SOI reads wrong in playtest, soi_factor is the knob
  (it was tuned for nominal radii; 4-5 on geometric radii lands near the
  old felt range).
