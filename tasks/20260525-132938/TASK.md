# Enforce: bevy_common_systems components never spawn entities directly

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.3.1, refactor, crates

Audit all components in bevy_common_systems and ensure they only attach to existing entities (parented pattern). Legacy #106.

## Resolution (v0.3.1, superseded)

Not applicable in this repo. bevy_common_systems is now an external crate
(github.com/alexjercan/bevy-common-systems) consumed as a git dependency; its
components are no longer part of this workspace, so the "components never spawn
entities directly" invariant can only be enforced in that repo. Closed as superseded
by the v0.3.0 externalization; tracked for the external crate instead.
