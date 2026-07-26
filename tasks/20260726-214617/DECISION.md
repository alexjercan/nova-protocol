# Decision: chin knob + SND state persists via the settings store

- DATE: 20260726-220100
- STATUS: ACCEPTED
- TASK: 20260726-214617
- TAGS: decision, ui, hud, settings

## Context

The BRIGHT/SCAN detents and the SND toggle need a home. Session-only resources
(reset on every game launch) would keep the work inside `nova_gameplay`;
persisting them means touching `nova_menu`'s settings store
(`crates/nova_menu/src/settings_store.rs`), which already round-trips
`MasterVolume` and `GraphicsQuality` through a RON blob (native config dir /
web localStorage) with serde-default tolerance for missing fields.

## Decision

Persist the monitor settings across game sessions through the existing
`PersistedSettings` store, exactly like master volume: new serde-defaulted
fields, snapshot on save, apply on load. SND defaults ON (owner call at the
same gate - the game has no browser-gesture constraint, and the chin button +
master volume are the opt-outs).

## Alternatives considered

- **Session-only resources** - simpler, single-crate; rejected by the owner at
  the 2026-07-26 plan gate: a player who dials in brightness/scanline depth
  should not redo it every launch.
- **A separate NOVA OS config file** - needless second store; the settings
  blob already tolerates added fields.

## Consequences

- Cross-crate wiring: the live resource lives in `nova_gameplay` (the drawer
  reads it every frame), while snapshot/apply live in `nova_menu` next to the
  existing volume wiring. The resource must be exported through the
  `nova_gameplay` prelude.
- Old settings files load fine (serde defaults); new fields must carry
  `#[serde(default = ...)]` with the POC-matching defaults.
