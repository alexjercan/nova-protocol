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

## Proposed shape

- **An UNBOUND turret becomes autonomous.** A mount with no player input binding
  auto-engages. Zero micro, decided at build time, and it staggers reloads
  naturally because the mounts do not fire in lockstep.
- Optionally, a BOUND turret auto-engages while the player is not firing it.
- **Avoid an all-or-nothing toggle.** It forces a choice between "no point
  defense" and "no manual control", when the point is to have both.

## Definition of done

- a player hull answers a salvo without the player aiming a single mount
- the player can still take manual control of what they bound
- ammunition spent per intercept measured for a player hull and compared against
  the AI figure the balance was set on
- a live playtest, with rendered evidence

## Scheduling note

Slotted into v0.11.0 at low priority because the gap is created by this
release's own torpedo work. Drop it to backlog if the release ships without it.
