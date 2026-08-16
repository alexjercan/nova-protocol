# A turret that cannot point at its target should not fire

- STATUS: IN_PROGRESS
- PRIORITY: 61
- TAGS: v0.11.0,combat,weapons,bug

## The complaint

Owner: "PDCs that cannot point to what they target should not fire, so if a PDC
is trying to shoot, but it cannot rotate to target it, it should not shoot,
sometimes I shoot while turning and I would say let's not do that... if the left
hand side turret is trying to target something on the other side of the ship, it
cannot fire at it duh".

## Confirmed in code

`shoot_spawn_projectile` (`crates/nova_ship/src/sections/turret_section/firing.rs:140`)
gates firing on:

- `WeaponsHot` (the safety)
- `SectionInactiveMarker`
- ammunition
- `TurretSectionInput` - the trigger bool

**It never checks where the barrel points.** So a turret fires the moment the
trigger is held, whether or not it has slewed onto anything, and a mount whose
hinges cannot reach the target fires forever into its own hull or into empty
space.

## The rule wanted

Fire only when the muzzle is ON the aim point, with SLACK. The owner: "only shoot
when PDC is locked on following the thing, or with some small threshold to not
have direct laser beams only, and allow some slack".

So this is an angular tolerance, not an exact match. Too tight and a moving
target is never engaged; too loose and the bug survives. Pick the threshold from
what the round can still hit at typical engagement range, and say how you chose
it - do not guess a number.

## What already exists

- `crates/nova_ship/src/sections/turret_section/aim.rs` - the aim solution
- `crates/nova_ship/src/sections/turret_section/arc.rs` - hinge limits and the
  reachable arc, including `hinges_to_first_muzzle`
- per-turret point defence already refuses to ASSIGN a target a mount cannot
  reach (`input/ai/point_defense.rs`), so the AI half of this rule exists. The
  firing half does not, and the player path has neither.

Prefer reusing the arc machinery over inventing a second reachability test.

## Watch for

- **A hard gate can silence a ship.** If a turret cannot bear, the ship should
  still be shooting with the turrets that can. Confirm the fleet does not go
  quiet.
- Ammunition is finite now, so this change SAVES rounds. Measure how many.
- The AI mirrors the player's firing path in places - check both.
- Do not break `fire_aligns_with_the_leaded_aim_point_not_the_anchor`, which pins
  that a turret fires at the lead point rather than the target's current
  position. A gate that compares the muzzle against the ANCHOR rather than the
  LEAD point would fight it.

## Definition of done

- a turret that cannot reach its target does not fire
- a turret mid-slew does not fire until it is within tolerance
- the tolerance is justified, not guessed
- a ship with one blocked mount still fires with the others
- rounds saved per engagement, measured
