# AI line-of-sight fire gate: hold fire and reposition when cover occludes the target

- STATUS: OPEN
- PRIORITY: 54
- TAGS: spike,v0.7.0,ai,gameplay,balance

Goal: make cover a real pressure-relief mechanic. AI turrets hold fire while
a tangible (non-Sensor) blocker occludes the aim point, and the ship uses its
existing approach/orbit machinery to regain the angle. This is NOT an AI
nerf: aim, lead prediction and damage are untouched; the AI stops wasting
ammo into rocks and visibly maneuvers for the shot. Today the fire decision
never raycasts (crates/nova_gameplay/src/input/ai.rs, fire gate ~:1391-1466),
so hiding behind an asteroid stops bullets (they expend on it,
turret_section.rs:431) but never stops the pressure.

Direction notes:
- Gate FIRING only, not target acquisition; dropping targets on occlusion
  would read as dumber AI and is out of scope.
- Recommend exempting point-defense (anti-torpedo) fire from the gate.
- Raycast via avian SpatialQuery from muzzle toward the LEADED aim point;
  mind perf (per firing turret, not per frame per turret if avoidable) and
  the two-clocks lesson (FixedUpdate vs render poses, docs/LESSONS.md).
- The mechanic is symmetric (enemies behind cover also stop eating player
  fire only if authored so) - decide and test the player-side effect.

Spike: tasks/20260717-111808/SPIKE.md (findings F3/F4; Options B)
