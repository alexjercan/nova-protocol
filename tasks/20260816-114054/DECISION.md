# Decision: the computer borrows idle PDCs

- DATE: 2026-08-16
- STATUS: ACCEPTED
- TASK: 20260816-114054
- TAGS: decision, combat, point-defence, input

## Context

AI ships already assign suitable PDCs across incoming torpedoes. Player ships can
only aim the whole battery manually. A combat lock follows one target, takes
about 1-2 seconds to acquire, and makes every PDC follow that target. This makes
manual defence against a multi-torpedo salvo an input and attention problem.

`tasks/20260815-231945/COMBAT-MODE.md` recommends permanent mount ownership: a
bound mount belongs to the player and an unbound mount belongs to the computer.
That solves automatic defence, but prevents the player from using every turret
for focused offensive fire. The owner rejected that restriction.

Nova already has the authority transition this feature needs:

- Hold RMB to raise the weapons and free-aim every PDC.
- Acquire a combat lock to keep the weapons active and aim every PDC at the
  locked target.
- With neither action active, the player is not using the PDCs.

## Decision

The computer borrows PDCs only while the battery is idle.

- No RMB raise and no combat lock: automatic point defence may assign and fire
  PDCs against incoming hostile ordnance.
- RMB raised: the player immediately owns every PDC. Existing free aim and LMB
  fire apply.
- Combat lock active: the player owns every PDC. Existing locked aim and LMB
  fire apply, including after RMB is released.
- Clear or release the manual state: the idle battery returns to automatic point
  defence.

Automatic point defence is not a toggle and is not a permanent property of a
mount. It is the fallback behavior of an idle battery. Every PDC remains
available for manual and locked fire.

Reuse the existing AI point-defence assignment behavior. Move or expose its
controller-agnostic targeting logic so player and AI ships use the same dwell,
reachability, threat allocation, and anti-overkill rules. Do not create a second
player-specific allocator.

## Alternatives considered

- **Permanent bound/unbound ownership.** Rejected: an autonomous mount becomes
  unavailable when the player wants the whole battery on one target. Binding
  configuration must not impose a point-defence-only weapon role.
- **Runtime auto-PD toggle.** Rejected: adds a persistent mode that can be
  forgotten and duplicates the authority signal already provided by RMB and
  combat lock.
- **Remove automatic point defence and use combat lock.** Rejected: one delayed
  lock drives the whole battery onto one torpedo. It does not provide the
  per-threat allocation needed for a salvo.
- **Balance changes in this decision.** Deferred. Existing AI mechanics are the
  baseline. Measure player behavior after the shared path works.

## Consequences

- The player can focus every PDC through free aim or combat lock.
- An unattended battery can defend against multiple incoming torpedoes without
  target micro-management.
- Manual authority and automatic authority are mutually exclusive for the
  battery. No player/AI target-precedence contest exists on one turret.
- Combat lock remains the focused-fire tool. Automatic defence targets hostile
  ordnance only.
- The transition must be immediate and visible enough that a player understands
  why turrets changed targets.
- Balance tuning, accuracy penalties, terminal-fire thresholds, and new input
  bindings are outside this decision.
