# Systems ranges: combat and destruction at both hull sizes

- STATUS: OPEN
- PRIORITY: 71
- TAGS: v0.14.0, testing, examples, combat

## Goal

Live proof for combat and destruction at BOTH hull sizes, plus the two
surfaces no range asserts at all: the mission HUD and the audio. From the
2026-09-09 coverage map: thirteen HUD invariants were all measured on one
4-section ship; torpedoes are proved only against asteroid gates; railgun
recoil has one sample on one mass; collision damage and the story HUD have
no live invariant; audio has 64 unit tests and zero live proof.

Rules: `examples/systems/README.md`. Each range gets its `[[example]]`
block and its roster slugs in `crates/nova_probe_cli/tests/catalog_drift.rs`,
and lands in the `combat` or `structure` shard of `20260909-213100`. A
range that finds a defect fixes it in the same lane and records it here.
The hull pair is `block_skiff` and `block_carrier` unless a range says why
not.

## Ranges

- [ ] `system_hud_scales`: the `system_hud_indicators` set replayed on a
      one-section shuttle and a capital hull at two window shapes.
      Invariants: every indicator stays inside the viewport on both hulls;
      one component marker per attached section at both sizes; the inset
      frames the whole hull; the lead pip stays on the intercept at capital
      scale; the velocity sphere does not swallow the ship. This is the
      proof for `20260909-212917` and `20260909-213350`.
- [ ] `system_collision_damage`: ram a hull into a rock and into another
      ship at three closing speeds. Invariants: a ram spends hit points on
      both bodies; the bite scales with closing speed; a touch below the
      threshold is free; a ram cannot damage its own sections; a ram that
      destroys a section leaves a wreck.
- [ ] `system_wreck_lock`: sever a component while the player holds a
      combat lock and a component pin on that ship. Invariants: the lock
      survives a section it was not pinned to; a pin on the severed
      component drops cleanly; the wreck becomes its own lockable contact;
      the reticle and inset follow what the lock ended on; the wreck still
      takes fire from the same gun.
- [ ] `system_torpedo_capital`: one bay firing at a large multi-section
      hull, then at a one-section drone. Invariants: the fuze beats the
      hull it is closing on; the warhead lands on the aimed section; a large
      hull absorbs the blast in the layer that was hit; a small target is
      not overflown; a target that dies mid-flight retires its torpedo.
- [ ] `system_railgun_hulls`: the shipped lance on a light hull and a heavy
      one. Invariants: recoil is inversely proportional to hull mass; a
      light hull is not spun by its own gun; the attitude ceiling absorbs
      the recoil torque; the reload holds one shell on both; the bore sight
      names where the slug went.
- [ ] `system_mission_hud`: a scenario posting objectives, markers, comms
      lines and a variable readout while the player flies. Invariants: an
      objective posts a chip and a marker; completing it feeds back once; a
      comms line names its speaker; the readout tracks the variable; an
      edge indicator points at an offscreen objective; the marker hides
      when its target dies.
- [ ] `system_ship_audio`: a hull firing, taking hits and burning, with the
      mixer buses driven from the settings resources. Invariants: a cue
      routes to the bus its authored track names; an exterior cue is placed
      by the listener; the per-source throttle collapses a burst; the
      master slider scales every voice; the engine hum tracks throttle; a
      dry trigger clicks once. Also the first live proof of the turret
      stow (`turret_section/stow.rs`) if it fits; else add it to
      `system_turret_gunnery`.

## Done when

Seven ranges green three times in a row in their shards, roster count
updated, every defect they found fixed and listed here.
