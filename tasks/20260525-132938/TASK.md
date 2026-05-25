# Enforce: bevy_common_systems components never spawn entities directly

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.3.1,refactor,crates


Audit all components in bevy_common_systems and ensure they only attach to existing entities (parented pattern). Legacy #106.
