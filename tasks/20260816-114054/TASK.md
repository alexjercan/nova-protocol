# Autonomous point defence for the player

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.11.0,combat,balance,ui,ship

## The gap

All point-defense code lives under `crates/nova_ship/src/input/ai/`, and
`input/player/` is a separate driver. Per-turret point defense - dwell,
hysteresis, and the "never assign a turret a target it cannot engage" rule - is
therefore a behaviour of the AI CONTROLLER. A player-controlled hull gets none
of it.

So the same salvo is answered automatically by an AI ship, and by the player only
if they manually swing each mount onto each torpedo. The owner ruled that out: "I
don't want to micro manage the PDCs, maybe an auto mode which scans for danger
and shoots at everything".

## Why it is a balance hole, not only a missing feature

Every number in `20260815-233950` assumes a defender that tracks perfectly and
never stops firing: 369 rounds an intercept against a weaving torpedo, one mount
answering 0.17 torpedoes/s, bay regeneration set to 59% of what two mounts
handle. A human answering by hand is far worse than that.

So a player currently fights an attrition economy balanced against point defense
they do not have. Finite ammunition sharpens it - inefficient answering now costs
rounds that cannot be got back inside an engagement.

## Most of the work already exists

`AITurretDefenseTarget` carries the dwell rule and the reachability check, and
the LOGIC is controller-agnostic - it sits under `ai/` for historical reasons,
not because it depends on the AI. Expect to lift it into a driver both
controllers share rather than to write new targeting.

## Accepted shape

See `DECISION.md`.

- **The computer borrows the battery while it is idle.** With no RMB raise and no
  combat lock, the existing point-defence logic may assign and fire player PDCs.
- RMB raise or combat lock immediately returns every PDC to the player and keeps
  the existing free-aim or locked-fire behavior.
- No mount is permanently point-defence-only. Binding presence does not decide
  ownership.
- No auto-PD toggle. Existing weapon authority determines the behavior.
- Reuse one controller-agnostic point-defence allocator for AI and player ships.
- Defer balance changes until the shared player path works and is measured.

## Definition of done

- an idle player battery answers a salvo without the player aiming a mount
- RMB raise immediately gives the player free-aim control of every PDC
- combat lock gives the player locked control of every PDC and suppresses
  automatic assignments
- returning to idle restores automatic point defence
- player and AI paths share the existing dwell, reachability, threat allocation,
  and anti-overkill behavior
- ammunition spent per intercept measured for a player hull and compared against
  the AI figure the balance was set on
- a live playtest, with rendered evidence of each authority transition

## Scheduling note

Slotted into v0.11.0 at low priority because the gap is created by this
release's own torpedo work. Drop it to backlog if the release ships without it.
