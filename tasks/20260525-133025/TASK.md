# Implement ammo limit logic

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0, weapons

Generic across turret and torpedo. Legacy #140.

Both weapon fire systems converge on the same shape: a fire-gate that spawns a
projectile when the trigger is held and a per-barrel/bay `Timer` is finished
(turret `shoot_spawn_projectile` gate at turret_section.rs:874; torpedo gate at
torpedo_section/mod.rs:504). Ammo adds a second gate + a decrement at those two
points, driven by one shared component so the two weapons stay generic. Absence
of the component keeps today's unlimited behavior (opt-in per config), which is
also what every headless physics test relies on.

Reloading and multiple ammo/bullet types (AP/EMP) are OUT of scope here and land
in task 20260708-162005; this is the single-pool foundation they extend. Keep
`SectionAmmo` shaped so a future reload/multi-magazine sits on top without a
rewrite.

## Steps

- [x] Add a generic `sections/ammo.rs`: `SectionAmmo { rounds: u32, capacity:
      u32 }` (Component, Reflect) with `new(capacity)`, `is_empty()`, and
      `try_consume() -> bool` (spends one round, false when empty). No systems -
      the two weapon fire systems own the decrement. Register the module in
      `sections/mod.rs` and re-export via its prelude.
- [x] Register the reflect type where sections are wired (a
      `register_type::<SectionAmmo>()`, matching the crate's register_type
      pattern, e.g. relations.rs:63).
- [x] Turret: add `ammo_capacity: Option<u32>` to `TurretSectionConfig`
      (default `None`), insert `SectionAmmo::new(n)` on the turret section in
      `insert_turret_section` when `Some(n)`. In `shoot_spawn_projectile` query
      `Option<&mut SectionAmmo>` on the turret: skip the whole turret when
      present-and-empty, and inside the `MAX_SHOTS_PER_TICK` loop `try_consume`
      before each spawn, `break`ing when it returns false so a magazine that
      empties mid-tick stops exactly at zero.
- [x] Torpedo: add `ammo_capacity: Option<u32>` to `TorpedoSectionConfig`
      (default `None`), insert `SectionAmmo::new(n)` on the section, and gate +
      decrement one round per launch in `shoot_spawn_projectile`
      (torpedo_section/mod.rs) alongside the fire-timer gate.
- [x] Tune capacities in nova_assets/sections.rs: give the turret and torpedo
      sections finite magazines (values are playtest knobs; comment them as
      such). Leave `ammo_capacity: None` on anything that should stay unlimited.
- [x] Tests (integration-first, headless): a turret with `SectionAmmo::new(k)`
      spawns exactly k bullets over a long hold then stops; try_consume unit
      behavior at the empty boundary; a torpedo bay fires exactly its capacity.
      A/B the fire-gate: same rig with no `SectionAmmo` keeps firing (proves the
      opt-in default and that the gate, not something else, stopped the stream).
- [x] Verify: cargo check + fmt; run the new tests. (Full clippy/suite runs in
      CI per repo convention.)
