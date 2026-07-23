# Goal: ch5 gravity round 2 - tiny wells, thrusterless base moved clear, keep AI out of wells

- DATE: 20260723
- UMBRELLA TASK: 20260723-223947
- LANDING SCOPE: squash-merge to master (local default branch), no push.

## Goal

Second round of ch5 gravity feedback after playtest: the RCS + leash base (task
20260723-200643) still gets pulled too strongly, and the AI fighters just fall
into the gravity wells - the AI is not smart enough to fly in a well yet. User's
call: "make the wells really small until we implement smarter AI."

So: stop fighting gravity with content tricks and instead keep the combatants
OUT of gravity entirely, with only tiny wells left as scenery the player brushes
on approach.

- Remove the base's RCS thrusters (they did not hold it) - back to a thrusterless
  AI base: it physically cannot move, so it holds station AND cannot chase (this
  also dissolves the earlier R1.1 "damaged base chases past its leash" risk - no
  thrusters, no chase).
- Move the base FURTHER from any well, and shrink every planetoid well so its SOI
  (`8 * radius`) is small.
- Keep the four AI raiders leashed tight to the base, well away from the (now
  small, relocated) wells, so no AI ship falls into one.
- The small wells sit only in the player's early approach corridor, so only the
  player (who can fly a well) ever encounters them.

The general "AI ships should handle gravity wells" work is filed as backlog task
20260723-224003; until then, wells stay tiny and combatants stay clear.

## Done means

1. The base carries NO thruster sections and no leash (`controller: AI(())`),
   keeps its 2 turrets, and is positioned so no planetoid SOI reaches it. (test:
   ch5 rig - base has 0 thrusters, 2 turrets; manual: it holds station.)
2. Every planetoid well is tiny (small `radius` => small `SOI = 8*radius`, low
   `surface_gravity`), and all planetoids sit in the early approach corridor,
   clear of the base and outside the raiders' leash reach. (cmd: lint clean;
   manual: nothing falls into a well.)
3. The four raiders are leashed tight to the base and never reach a planetoid
   well. (manual: the fighters stay in the base fight, not captured by gravity.)
4. Bundle version bumped, docs synced (CHANGELOG; the 1.11.0 RCS entry stays as
   dated history). (cmd: lint clean.)

Overall: targeted check suite passes; a playtest shows the base holding and no
AI ship falling into a well.

## Tasks

- [ ] 20260723-223947 (p0, umbrella) this goal
- [x] 20260723-223954 (p62, the-ledger) ch5 gravity r2 (content)
      landed 881273f7; 1 review round (APPROVE, out-of-context; R2.1 NIT - imprecise
      "piloted" phrasing, no change); bundle 1.11.0 -> 1.12.0; base thrusterless
      + moved clear of tiny wells, raiders leashed 200 short of every well
- (backlog) 20260723-224003 smarter AI for gravity wells (separate future work)

## Decisions (load-bearing, architectural)

- Reverts the RCS-station-keep decision from 20260723-200643 (SUPERSEDED). An
  armed AI ship chases when it engages and the `recently_damaged` override lets a
  shot ship exceed its leash, so RCS + leash could not reliably hold the base
  that is itself the target. With the AI unable to fly wells at all, the robust
  answer is to keep all combatants OUT of gravity (thrusterless base + tiny,
  relocated wells) rather than have content fight the physics. Smarter AI
  (backlog 20260723-224003) is the real fix; this is the hold-until-then.

## Manual acceptance (batched for the user at Finish)

All verifiable done-definition items MET + re-verified on master. This is the
human-playtest gate (ch5 is un-hidden for direct testing). Any issue -> new task.

- (pending user playtest) the base holds its position; NO AI ship (base or the
  four fighters) falls into a gravity well; the wells are gentle scenery on the
  approach; torpedoes still on R; launches from the picker.

## Finish (2026-07-23)

Task 20260723-223954 landed 881273f7 (review APPROVE r1; one NIT, R2.1 - an
imprecise "piloted" shorthand in the planning prose, no code change). Master
re-verified green: content lint 0 err/warn/finding (6 scenarios balance-audited);
`ledger_ch5_raid` 11/11, `ledger_ch4_ending` 10/10, `webmods_validation` 1/1.

Done-definition on master (all MET): (1) base thrusterless (`AI(())`, 0 thrusters,
no leash), 2 turrets, parked at (0,15,-580) clear of every well (360-449u vs SOI
64-72); (2) tiny wells (radius 8-9, gravity 1) in the early approach; (3) raiders
leashed 200 and 96-185u short of any well - none fall in; (4) bundle 1.12.0 +
docs synced. The playtest feel is the deferred user gate above.

Conformance: `tatr check` clean on both tasks; `tatr check --ledger LESSONS.md`
clean. `/lessons`: `/compound` folded this cycle's lessons (renamed/sharpened
`read-all-branches-of-a-load-bearing-engine-rule` x2, and the
`bundle-version-string-pin-bites-on-bump` x2 lesson paid off preventing its own
recurrence); no loose docs/ scratch; the docs/ wipe defers to the 0.8.0 release.

Residue: none dropped. The real fix for the underlying limitation (AI ships
cannot fly gravity wells) is filed as backlog task 20260723-224003, so the wells
can grow back once that lands - explicitly deferred, not silently omitted.

REMINDER (still standing): ch5 is `hidden: false` for testing - re-hide it before
the 0.8.0 release so the raid stays a fight-only reward.
