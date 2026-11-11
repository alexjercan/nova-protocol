# Feedback polish for docking, pickup, railgun hits, kills and sector crossing

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.18.0,polish,audio

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction:
apply the torpedo feedback treatment (light, trail, sound, notice, stinger)
to the other bare moments.

## User facts
- v0.17.0 torpedo chain is the reference quality bar.

## Agent findings (polish inventory, bare spots)
- Docking and undock: port sleeve animation only
  (`crates/nova_ship/src/sections/docking_section/render.rs:1-8`); no sound
  anywhere, `ship_audio/machinery.rs:3-5` does not cover ports; no DOCKED
  notice or flash.
- Canister pickup / cargo intake: sounds exist (`salvage_pickup`,
  `cargo_eject`, `bay_door`) but no visual and no HUD flash.
- Sector crossing: no visual, no sound, no notice.
- Railgun: slug, tracer, wake and muzzle light exist
  (`railgun_section/wake.rs`, `render.rs:296`) but impact is the shared PDC
  spark (`crates/nova_gameplay/src/impact_spark.rs`). No ricochet visual or
  sound for PDC rounds.
- Kill stinger plays only for torpedo kills (`assets/base/sounds/README.md:136`).
- Station lights: none; lights exist only as authored scenario objects
  (`crates/nova_scenario/src/objects/light.rs`).
- Fine already: section destruction, asteroid carve, mining beam, thruster
  burn, RCS puffs.

## Decisions (pending owner review)
- Scope in priority order: docking clamp sound + DOCKED/UNDOCKED notice +
  port light; intake flash and a HUD count tick on pickup; railgun impact
  (penetration flash and hull scorch decal or spark variant) and a PDC
  ricochet sound; kill stinger for any kill by the player; sector crossing
  cue (chart ping + short notice, no screen effect); generated station
  navigation lights.
- Every new sound is optional content on the section or presentation, named
  like the torpedo bay sounds (`ignition_sound`, `intercept_sound`), and a
  mesh or sound ref that names no file fails lint.
- Low graphics keeps the sounds and the notices and drops lights and decals,
  matching the torpedo trail rule.

## Delivery
- Owners: `nova_ship` docking_section and railgun_section render,
  `nova_gameplay` impact_spark and juice, `nova_hud` notices, `nova_world_base`
  station layout for lights, `nova_authoring` sound fields. Dies: nothing.
  May break: sound README inventory, lint for presentation sounds, low-end
  probe frame budgets if lights land on stations.
- Sequence: docking -> pickup -> railgun and ricochet -> any-kill stinger ->
  sector crossing -> station lights. One commit per moment.

## Verification
- Asserted examples per moment: event fires once per occurrence (one dock =
  one clamp, one pickup = one tick), notice text and timing, Low keeps sound
  and drops the light.
- Capture sheet for docking and railgun impact under NOVA_CAPTURE for the
  visual claims; compare against the v0.17.0 torpedo sheet.
- No timing assertions.

## Done when
- Docking, pickup, a railgun hit, any player kill and a sector crossing each
  have a sound and a visual or notice at High, and a sound at Low.
- Sound README and wiki list the new optional fields.
