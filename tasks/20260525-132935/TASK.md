# Audit and finalize bevy_common_systems crate boundary

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.3.1,refactor,crates


Audit what currently lives in crates/bevy_common_systems and confirm it only contains general-purpose, game-agnostic helpers (thruster, PD controller, shared physics). Move any nova-specific code out. Refs legacy #144, #145, #133.
