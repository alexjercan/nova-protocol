# RETRO: neutralized (combat-dead) ship state (20260722-092320)

Landed the critical-damage feature: an armed ship that loses all working
weapons AND thrusters becomes NEUTRALIZED - a distinct inert-wreck state (new
`OnNeutralizedEvent`, `NeutralizedMarker`, AI off, not despawned), with the
player getting an immediate Defeat. Absorbed the sibling kill-condition
annoyance (20260722-092326): a beaten ship now counts as beaten without
grinding every hull section to zero.

## What went well

- Two rounds of targeted exploration BEFORE any code turned a fuzzy "spike
  this" task into three concrete, mutually-exclusive forks. Mapping the exact
  seam (scenarios have NO central beaten-registry; each reacts to
  `OnDestroyedEvent` by id) is what let me pose Fork 3 as a real decision
  instead of guessing.
- The three forks went to the user as decisions, not inferences - and the
  answers materially changed the build (distinct event + per-scenario mirroring
  vs a central bridge). Recorded in DECISION.md before building.
- The armed-at-spawn guard (`WasArmedCombatant`) fell out of a correctness
  question (an unarmed hauler losing engines would auto-complete "destroy the
  hauler") and ALSO solved the post-spawn/sections-not-yet-attached false
  positive and made the scenario churn self-consistent (unarmed friendlies need
  no sibling and cannot regress). One guard, three problems.
- Test-first-ish at two altitudes: integrity unit tests for the predicate/guard/
  wreck, and production-faithful scenario tests that load the REAL shipped RON
  and drive the REAL handlers (mirroring broadside_assault.rs) - not a hand-built
  rig (`review-rig-can-false-green` avoided).
- Reusing the existing `act`-guards for once-semantics: a neutralize advances
  the act, so the later real `OnDestroyed` win/defeat gate simply no longer
  matches. No new double-fire machinery needed.

## What went wrong / friction

- I nearly hand-edited scenario siblings by classifying each target's armament
  from the RON, but section kinds live in prototype content, not the scenario.
  I switched to "mirror every enemy kill-objective; the arming guard makes an
  unarmed sibling harmless dead code" - but then verified (via the reviewer)
  that the shipped unarmed set really is unarmed, so I kept the siblings
  minimal (armed-only). The lesson: when you can't cheaply classify at the edit
  site, prefer the robust-superset edit OR push the classification to a verifier;
  don't eyeball it.
- One process slip: I amended DECISION.md in the MAIN checkout (on master)
  mid-work instead of in the worktree, creating an uncommitted master edit. Caught
  it, discarded it there, re-applied in the worktree. Task-record edits during
  work belong on the BRANCH (see `commit-review-retro-before-land`).
- The `.after(IntegritySystems)` ordering is weaker than it reads:
  `SectionInactiveMarker` is written by an OBSERVER, not a system in that set, so
  detection can be one frame late. It is safe (late, never false), but the
  original comment overclaimed same-frame semantics - the reviewer flagged it and
  I corrected the comment.

## Lessons for the ledger

- `classify-at-the-verifier-when-the-edit-site-cant`: when a bulk content edit
  needs a per-item property (here: is this ship armed?) that is NOT visible at the
  edit site (section kinds live in prototype content, not the scenario RON),
  either make the robust superset edit whose extra items are provably harmless, or
  hand the classification to a verifier/reviewer - do not eyeball it per item.
- `guard-timing-matches-observer-not-set`: a Bevy run-ordering `.after(SomeSet)`
  does NOT order past state written by OBSERVERS fired from that set's commands;
  `SectionInactiveMarker` (glue observer) is visible only next frame. Design the
  predicate to be safe under one-frame-late inputs (absent => "working") rather
  than assuming same-frame ordering, and say so in the comment.
- (reinforces `commit-review-retro-before-land`): task-record edits made during
  work must happen in the WORKTREE, never the main checkout - an amendment on
  master is a leak waiting to be swept.
