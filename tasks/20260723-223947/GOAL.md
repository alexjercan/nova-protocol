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
- [ ] 20260723-223954 (p62, the-ledger) ch5 gravity r2 (content)
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

- (pending) playtest ch5: the base holds its position; NO AI ship (base or the
  four fighters) falls into a gravity well; the wells are gentle scenery on the
  approach; torpedoes still on R; launches from the picker.
