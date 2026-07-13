# Retro: Blast collision fires inconsistently (ordering bug)

- TASK: 20260706-162912
- BRANCH: fix/blast-collision-ordering
- PR: #40 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260706-162912/TASK.md`. A FIXME that turned out to be a real, explainable
dispatch-semantics bug once the engine internals were read.

## What went well

- Read the engine, not just the symptom. The FIXME framed it as a mystery ("I don't know what
  the problem might be"). The answer was one function in avian - `trigger_collision_events`
  raises `CollisionStart` once per collider in the pair that has `CollisionEventsEnabled`, with
  that collider as `body1`. Reading it turned "fires inconsistently" into a precise, provable
  cause: the blast never enabled events, so it depended on the target's.
- Used the FIXME's own hint. It pointed at `area.rs` as the thing that "works both ways".
  Reading `area.rs` showed it does NOT handle both orderings either - it just owns its events.
  That is the whole fix, and copying an established in-repo pattern beats inventing one.
- The 170001 harness paid off immediately. The physics test harness merged one task ago made
  the regression test cheap: spin up a headless avian world, spawn a blast + a target, assert
  damage. No new infrastructure.
- Wrote a regression test that actually distinguishes fixed from broken. Constructing a target
  that genuinely lacks `CollisionEventsEnabled` (collider spawned without `Health` so the enable
  observer skips it) is what makes the test fail on the old code and pass on the new - a test
  that just fired a normal blast at a normal target would have passed both ways and proved
  nothing.
- Checked the interaction, not just the unit. Enabling events on the blast means the impact
  observer now also sees blast pairings; asserting the falloff test deals *exactly* 60 (not
  more) proved the impact path still early-returns for the static blast, so the change did not
  leak spurious impact damage.

## What went wrong

- Spent a while theorising that the bug "couldn't actually manifest" because every Health-
  bearing target auto-enables events. That was true but beside the point: the fix is about not
  *depending* on that invariant. Root cause of the detour: chasing "when does this break today"
  instead of "what is the correct ownership of the event". The design answer (the emitter owns
  its events) was reachable straight from `area.rs` without the manifestation hunt.

## What to improve next time

- For an event/observer "fires inconsistently" bug, read the emitter's dispatch rule first -
  who raises the event, keyed on what, with which entity as the target. Physics/observer events
  often fire per-participant, not per-pair, and half the observer bugs are an ordering
  assumption that only holds when one specific side opted in.
- When a FIXME cites a sibling that "works", read that sibling before theorising - it usually
  encodes the intended pattern.

## Action items

- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro) - trivial, for whoever next edits that file.
