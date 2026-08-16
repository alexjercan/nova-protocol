# A neutralized ship stops fighting, including its point defence

- STATUS: IN_PROGRESS
- PRIORITY: 62
- TAGS: v0.11.0,combat,ai,bug

## The bug

Owner: "neutralized ships should not fire at flying torpedoes, they should be
simply 'deactivated' (neutralized means dead but not completely destroyed so no
more ship crew or AI)".

A neutralized hull is a wreck with nobody aboard. It must not shoot.

## The inconsistency, already located

Target acquisition RESPECTS neutralization - `crates/nova_ship/src/input/ai/acquisition.rs:147`
reads `Has<NeutralizedMarker>`.

Per-turret point defence does NOT. `crates/nova_ship/src/input/ai/point_defense.rs`
selects ships with `(With<SpaceshipRootMarker>, With<AISpaceshipMarker>)` at `:79`
and `:132` and never excludes `NeutralizedMarker`. So a dead hull's mounts keep
tracking and killing torpedoes.

Point defence landed this release, which is why it missed the rule the rest of
the AI already follows.

## Do not stop at point defence

The owner's framing is broader than one query: neutralized means the crew is
gone. So AUDIT every AI behaviour for the same omission rather than patching the
one symptom - guns, torpedo launch, manoeuvre, comms, target acquisition,
anything keyed on `AISpaceshipMarker`.

There is an `On<Add, NeutralizedMarker>` observer at
`crates/nova_ship/src/input/ai/mod.rs:223`. Decide deliberately whether the right
fix is to widen that observer (deactivate once, at the moment of neutralization)
or to add `Without<NeutralizedMarker>` to each query. **Prefer the observer if it
can carry the whole rule** - one place that says "the crew is gone" is better
than N queries that each have to remember.

Report which behaviours were already correct and which were not. The list is the
valuable part.

## Watch for

- A neutralized ship is still SOLID and still takes damage. Deactivating the crew
  must not make the hull intangible or invulnerable.
- Scenario scripts count neutralized ships (`OnNeutralizedEvent`,
  `DefeatedMarker`). Do not change what those report.
- The player can be neutralized too (`acquisition.rs:672` inserts the marker on a
  player). Confirm whatever you change behaves for a neutralized PLAYER hull, not
  only an AI one.

## Definition of done

- a neutralized hull does not fire at torpedoes, and does not fire at all
- the audit list: every AI behaviour, whether it already honoured neutralization
- a live run showing a neutralized ship going quiet while a torpedo flies past it
- existing neutralize tests still pass, and the counts scenarios read are unchanged

## Lane

sprout `neutralized-quiet`.
