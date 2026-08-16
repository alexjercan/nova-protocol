# Autonomous point defence for the player

- STATUS: CLOSED
- PRIORITY: 53
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

## Approved plan (2026-08-16, supersedes the open questions above)

The owner resolved the mode-vs-fitting debate as an OWNERSHIP PRECEDENCE per
mount, granted by a new controller FlightVerb:

1. Precedence, per turret: player lock (CTRL) -> manual combat control ->
   Flight Computer point defence -> cold. Releasing a mount returns it to the
   computer after a short debounce grace.
2. New `FlightVerb::PointDefense`: granted by default, withholdable per
   controller/scenario via the existing DisableVerb modification and
   SetControllerVerb action - the teaching lever, and forward-compatible with
   the owner's buy/skill-level idea for verbs.
3. Reuse: the per-turret torpedo assignment AI ships already run claims the
   player's IDLE mounts; the 0.92 deg bearing gate and dt-invariant aim are
   already landed.
4. Visibility: a thin gizmo line from each computer-held PDC to its target.
5. Verification: precedence-transition lib tests plus an example emitting the
   probe JSONL.

## Lane

pd-verb (opus). tasks/20260815-231945/COMBAT-MODE.md is background reading;
this task does NOT build a combat mode - it leaves a named seam where an RMB
combat mode plugs into the same precedence.

## Closure

Landed 2026-08-17, lane pd-verb (opus). The approved plan shipped whole:

- FlightVerb::PointDefense granted by default, withheld via the existing
  DisableVerb / SetControllerVerb machinery; fail-open on hulls with no
  controller so bare rigs keep defending.
- MountAuthority per turret (PlayerLock > PlayerManual > FlightComputer >
  Cold) resolved by one pure fn; the RMB combat mode seam is a named doc
  comment on PlayerManual.
- Regrasp grace 0.5 s, written as 2 * RADAR_TAP_SECS so the derivation is
  executable: a tap-clear or an RMB gap must not swing the battery off and
  back inside one gesture.
- The AI point-defence allocator moved to input/point_defense/ and claims
  idle player mounts; weapons-safety exempts computer-worked mounts (idle ==
  cold, nothing would ever fire otherwise); a latched trigger releases on
  authority loss so no shot leaks into the player's hands.
- Thin amber gizmo line (0.75 px, HUD gunnery family) muzzle-to-torpedo,
  only for computer-held mounts, gated on the render flag.
- Evidence: borrowed_battery driven range emits the full JSONL beat chain -
  claim at t=8.3, cold-hull shot inside the 0.92 deg gate at t=9.4, player
  lock stealing the mount, return after exactly the grace. 12 new lib tests
  pin every transition. Probe snapshots now carry per-turret "authority".

Found in passing, NOT fixed (pre-existing): examples/systems/
neutralized_quiet.rs flakes 4/4 on base commit 5278e8f2 - its step reads
assignment+firing a frame after advancing on it and the AI trigger drops for
a frame while the barrel slews. Will fail CI on a fast box; owner's call.
