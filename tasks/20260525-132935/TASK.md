# Audit and finalize bevy_common_systems crate boundary

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.3.1, refactor, crates

Audit what currently lives in crates/bevy_common_systems and confirm it only contains general-purpose, game-agnostic helpers (thruster, PD controller, shared physics). Move any nova-specific code out. Refs legacy #144, #145, #133.

## Resolution (v0.3.1, superseded)

Not applicable in this repo. The v0.3.0 cleanup extracted bevy_common_systems into
its own standalone repository (github.com/alexjercan/bevy-common-systems), consumed
here as a pinned git dependency. The crate no longer lives in crates/, so its internal
boundary can only be audited in that repo. The boundary work for this workspace is
complete: nothing bevy_common_systems remains to move out. Any residual internal audit
belongs to the external crate's own backlog. Closed as superseded by the externalization.
