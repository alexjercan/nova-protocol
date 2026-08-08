# Extract torpedo into its own module/plugin; unhardcode blast params

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.4.0, torpedo, refactor

From the TODO sweep (task 20260525-132954). The torpedo logic and its targeting system
live inline in torpedo_section.rs and should be factored into their own module/plugin;
blast parameters (radius, damage) are hardcoded and should be config-driven.

Source TODOs (crates/nova_gameplay/src/sections/torpedo_section.rs):
- Factor out the torpedo logic into a separate module
- Implement a separate plugin for the targeting system
- Unhardcode blast parameters

## Resolution

Two parts, behavior-preserving.

1. Module split. `torpedo_section.rs` (~1300 lines) became a `torpedo_section/`
   directory (module name unchanged, so the public path `torpedo_section::*` and
   `TorpedoSectionPlugin` are untouched):
   - `mod.rs` (~510) - config, plugin, prelude, all components, and the bay/launcher
     (spawn + fire-rate + `shoot_spawn_projectile`), plus the tests.
   - `projectile.rs` (~250) - in-flight behavior: target tracking, arming, detonation,
     PN guidance (steer / sync / thrust). Systems are `pub(super)` so the plugin and
     tests in `mod.rs` reach them.
   - `render.rs` (~225) - the render/particle observers.
   Removed the stale factoring TODOs. The "separate plugin for the targeting system"
   TODO is dropped as-is: torpedo target *selection* already lives in the input plugin
   (`input/player.rs`); the projectile-side `update_target_position` (just position
   tracking) belongs with the other flight systems in `projectile.rs`, and a one-system
   plugin would be ceremony.

2. Unhardcoded blast params. `BLAST_RADIUS` / `BLAST_DAMAGE` consts are gone; added
   `blast_radius` (30) and `blast_damage` (100) to `TorpedoSectionConfig`, carried onto
   the projectile via a new `TorpedoBlast { radius, damage }` component that
   `torpedo_detonate_system` reads. The in-game section config sets them.

Verified: 29 nova_gameplay tests pass, clippy clean, examples 06/07 build; headless
smoke of both is byte-for-byte the same behavior (06: 3 fired/armed/detonated, 07: 2
detonations, no panic).
