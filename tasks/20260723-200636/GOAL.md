# Goal: ch5 raid playtest tuning - gravity, station-keeping, turrets, torpedo key, unhide

- DATE: 20260723
- UMBRELLA TASK: 20260723-200636
- LANDING SCOPE: squash-merge to master (local default branch), no push. Default
  flow landing.

## Goal

Playtest feedback on the just-landed ch5 raid (task 20260723-182855): the
planetoid gravity is too strong for the small ships and drags the space station
off its position, there are too many turrets on the base, the torpedoes should
be on the R key, and the chapter needs to be temporarily un-hidden so the user
can launch it directly to test.

Root causes found in the engine (not guesses):
- Gravity SOI radius = `soi_factor (8) * body_radius`, so the planetoids (radius
  18-26) have 144-208u wells; the base at (0,15,-520) sits ~116u inside
  planetoid_3's 208u well and is dragged. Pull strength `mu = surface_gravity *
  radius^2` (capped at 10), so radius 26 + gravity 7 is a very strong, wide well.
- Only PILOTED ships feel gravity (`fix(gravity): only piloted ships feel
  gravity wells`); the base is an AI ship with NO thrusters, so it feels the pull
  but has no RCS authority to resist it.
- An armed AI ship that ENGAGES runs the chase/aim/fire pipeline
  (`AIBehaviorState::Engage`), so giving the base thrusters would make it FLY AT
  the player, not hold station. The robust "holds station and still shoots" is a
  THRUSTERLESS armed AI ship (it cannot chase, so it holds position; its turrets
  still aim/fire) placed OUTSIDE all gravity wells (so nothing drags it).

## Done means

1. Planetoid gravity is calmer and the base sits in gravity-free space: each
   planetoid's `surface_gravity` and `radius` are reduced and repositioned so
   NO planetoid SOI (`8 * radius`) reaches the base spawn, and the approach
   corridor is flyable for the small ships. (cmd: content lint clean; manual:
   playtest - small ships handle the field, base does not drift.)
2. The base holds its position via RCS + a tight leash: it gains thruster
   sections and `leash: Some(15.0)`, kept AI + Enemy, placed in mild gravity at a
   safe distance so RCS holds it and the leash keeps it from chasing. (test: the
   ch5 rig shows the base spawns Enemy AI with thruster sections and a ~15u
   leash; manual: it stays put and does not chase.)
3. The base has FEWER turrets (4 -> 2). (test: ch5 rig asserts the base turret
   count; cmd: lint clean - mounts still valid.)
4. Torpedoes fire on the R key (keyboard), not RMB; gamepad stays RightTrigger2.
   (test: ch5 rig asserts the torpedo cubes bind the R key; cmd: lint clean - no
   flight-rig key conflict.)
5. ch5 is `hidden: false` so it can be launched from the Scenarios picker for
   testing. (test: ch5 rig asserts it is not hidden; manual: it appears in the
   picker.) NOTE: temporary test state - flag re-hiding before the release.
6. Bundle version bumped, docs synced (CHANGELOG, README/news torpedo-key
   mention, mod-guide version walk). (cmd: lint clean.)

Overall: the full targeted check suite passes, and a playtest confirms calmer
gravity, a base that holds station, and the R-key torpedoes.

## Tasks

- [ ] 20260723-200636 (p0, umbrella) this goal
- [x] 20260723-200643 (p60, the-ledger) ch5 raid tuning (content)
      landed 9543e39f; 1 review round (APPROVE, out-of-context; R1.1 MINOR - the
      recently-damaged leash override, a disclosed playtest item); bundle 1.10.0
      -> 1.11.0; gravity shrunk, base RCS+leash+2 turrets, torpedoes on R, ch5
      un-hidden for testing

## Decisions (load-bearing, architectural)

- Base station-keeping via RCS + a TIGHT LEASH at a safe distance (the user's
  call at the gate: "place it such that RCS would work, safe distance"). The
  base gets thruster sections (RCS authority) + its controller core + AI, placed
  in MILD (reduced) gravity so RCS has authority to hold, at ~610u from the
  player. The chase-when-engaging problem is bounded by a tight `leash` (~15u):
  `AILeash` centers on the spawn position and, beyond its radius, "combat breaks
  off and the tether reasserts" - so the base can only nudge a few units toward
  the player before it is pulled home. Net: it station-keeps via RCS and cannot
  wander off. (Chosen over the thrusterless/out-of-gravity alternative because
  the user wants the RCS hold to be real.)

## Manual acceptance (batched for the user at Finish)

All verifiable done-definition items (1-6) are MET and re-verified on master (see
Finish). These are the human-playtest gate - the user is set up to test directly
now that ch5 is un-hidden. Any playtest issue becomes a new prioritized task.

- (pending user playtest) gravity feels manageable for the small ships; torpedoes
  fire on R; the base has a lighter turret load (2); the chapter launches from
  the picker.
- (pending user playtest, R1.1) the base holds its position AND does not walk
  toward you once your torpedoes start landing - the engine's `recently_damaged`
  tether override can let a shot ship exceed its leash. If it drifts/lurches, the
  levers are: tighten the leash, lower planetoid_3 gravity, or move the base out
  of the well entirely.

## Finish (2026-07-23)

Task 20260723-200643 landed 9543e39f (review APPROVE r1; one MINOR, R1.1, a
disclosed playtest item). Master re-verified green: content lint 0
err/warn/finding (6 scenarios balance-audited); `ledger_ch5_raid` 11/11,
`ledger_ch4_ending` 10/10, `webmods_validation` 1/1.

Done-definition on master (all MET): (1) planetoids shrunk, base in only a mild
residual well (geometry computed + reviewer-reproduced: p3 99u vs SOI 128, accel
~0.08 u/s^2); (2) base holds via 2 RCS thrusters + leash 15 (rig-pinned); (3) 2
turrets (rig-pinned); (4) torpedoes on R (rig-pinned exact binding, no flight
conflict); (5) `hidden: false` (rig-pinned, re-hide comment); (6) bundle 1.11.0 +
docs synced. The playtest feel (incl. R1.1) is the deferred user gate above.

Conformance: `tatr check` clean on both tasks; `tatr check --ledger LESSONS.md`
clean. `/lessons`: per-task `/compound` folded this cycle's lessons
(`bundle-version-string-pin-bites-on-bump` x2, `read-the-override-branch-of-a-bounding-mechanism`
x1); no loose docs/ scratch; the docs/ wipe defers to the 0.8.0 release.

Residue: none dropped. R1.1 is carried as a manual-acceptance/playtest item above
(not silently closed). A known shared-checkout event: a parallel session landed
`f29c2727 docs: update tasks` (the unrelated pre-existing file) mid-cycle; merged
cleanly into the branch before landing, no overlap with this work.

REMINDER for the user: ch5 is temporarily `hidden: false` for testing - re-hide
it (a one-line flip back to `hidden: true`) before the 0.8.0 release so the raid
stays a fight-only reward.
