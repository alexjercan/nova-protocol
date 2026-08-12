# NOTES

## Current model

- `IntegrityDestroyMarker` is the destruction seam. Its observers fire
  `OnDestroyed`, create debris, and despawn the destroyed entity.
- `OnDestroyed` does not itself request despawn. Event emission and despawn are
  sibling reactions to the same destruction marker, so they are still
  inseparable in current gameplay semantics.
- `NeutralizedMarker` means an armed ship has no working weapon or thruster.
  It fires `OnNeutralized` once and leaves the physical ship in the world.
- Neutralized AI gains `AINonCombatant`, which clears its own target and stops
  it fighting.
- Neutralization does not change allegiance. Presentation preserves allegiance
  color, while threat collection and AI target acquisition exclude wrecks.
- Scenario kill objectives duplicate `OnDestroyed` and `OnNeutralized` because
  there is no single scenario-level "ship defeated" transition.

## Candidate state model

```text
Active combatant
  -> Neutralized wreck -> Physically destroyed
  -> Physically destroyed
  -> Withdrawn / escaped
```

- Neutralized: defeated, persistent, powerless wreck.
- Destroyed: physical object destruction and removal.
- Withdrawn: left the engagement; not automatically defeated or neutralized.
- Scripted despawn / scenario teardown: cleanup, not destruction.

A ship should report defeat exactly once on either neutralization or direct
physical destruction. Later destruction of an already-neutralized wreck must
not report defeat again.

## Candidate event split

- `OnDefeated`: scenario outcome edge for a ship that is no longer a combatant.
  Fires once for neutralization or direct destruction.
- `OnNeutralized`: optional detailed edge for transition into a persistent
  wreck, if scripts need that distinction.
- `OnDestroyed`: physical destruction edge. It can follow neutralization.
- No destruction event for scripted despawn, teardown, or boundary cleanup.

Direct destruction ordering would be `OnDefeated` then `OnDestroyed`.
Neutralized-later-destroyed ordering would be `OnDefeated`, `OnNeutralized`,
then later `OnDestroyed`.

## Feedback direction

- Neutralized ships must stop appearing as active threats.
- Exclude them from `ThreatContacts` and AI target acquisition. `ThreatContacts`
  is the player's ranked set of hostile combat targets used for off-screen edge
  indicators; it is separate from the held `CombatLock`.
- Keep an existing combat lock on a neutralized wreck. Change its presentation
  instead of clearing it.
- Give neutralized enemies a distinct marker state, starting with a hollow red
  wreck chevron instead of the active solid red triangle.
- Show `NEUTRALIZED` in target details.
- Add transient `NEUTRALIZED` / `DESTROYED` confirmation near either the target
  inset or screen center; choose the exact surface after inspecting both.
- The wreck remains physically targetable, but missions must never require
  chasing it for a final hit.

## Implemented feedback slice

- Neutralized wrecks are excluded from player `ThreatContacts` and AI target
  acquisition.
- Existing and new combat locks remain valid.
- The world marker swaps its solid allegiance triangle for a hollow chevron and
  preserves allegiance color. Enemy wrecks therefore show a hollow red V.
- The target inset caption replaces the relation tag with `NEUTRALIZED` while
  retaining the target name and allegiance color.
- A 1.4-second `NEUTRALIZED` confirmation flashes inside the target inset only
  when the player's held target changes state. Inset-local placement avoids the
  busy combat bracket and section markers at screen center. Unlocked ships
  defeated by AI do not produce player-facing credit.
- `screenshot_combat` captures `variant-neutralized-wreck.png` as a rendered
  presentation pin for the chevron and target details. Its live battle can
  destroy the raider first, so it does not stage the transient confirmation.
- Manual combat playtest confirms the inset-local transient appears for a
  neutralized held target.
- Physical destruction of the framed lock records `IntegrityDestroyMarker`
  before despawn, then holds an amber `DESTROYED` ribbon for the two-second kill
  cam. Generic despawn, scripted cleanup, and unrelated destruction do not
  trigger it. Destruction supersedes a recent neutralization flash.

## Boundary direction

Do not infer neutralization from crossing a scenario border. A functional ship
that leaves is withdrawn or escaped, which is a mission-specific outcome.
Borders can stop engagement and clean up distant entities, but should use a
separate withdrawn/out-of-bounds state. Cleaning up an already-defeated wreck
must not fire `OnDestroyed`.

## Open decisions

- Whether `OnDefeated` replaces authored `OnNeutralized`, or both remain.
- New combat locks can be acquired on neutralized wrecks. Existing locks stay
  held. Both use distinct neutralized presentation.
- Whether wrecks coast forever, receive damping, or clean up after distance or
  time.
