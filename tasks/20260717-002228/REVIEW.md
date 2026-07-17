# Review: Let sections declare sounds + move base sounds under assets/base/

- TASK: 20260717-002228
- BRANCH: task-20260717-002228-section-sounds

Reviewed the committed diff (8f97da1d) with fresh eyes plus an independent
out-of-context pass (subagent over `git diff master...HEAD`). Load-bearing claim
independently re-verified: base bundle `resource_base = "base"` (nova_assets
lib.rs:254, read_bundle("base", ...) at :1393), so `self://sounds/turret_fire.wav`
rewrites to `base/sounds/turret_fire.wav` - byte-identical to what
`register_sounds` loads via `format!("base/sounds/{name}.wav")`, so base playback
is genuinely unchanged (same asset path -> same handle). The passing
`content_lint_gate` independently confirms resource membership.

Verification run (this worktree): nova_gameplay lib 520 passed; nova_assets
mod_refs 19 passed; content_ron_parity + content_lint_gate 4 passed;
`cargo check --workspace --all-targets --features debug` clean (one pre-existing
unused-`mut` warning in examples/19_broadside.rs, not in this diff). Both turret
configs wired; all 16 sounds in base resources; no stray `assets/sounds/` refs.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/sections/turret_section.rs:449-452,
  577-585 - `insert_turret_section` gained an unconditional `Res<AssetServer>` to
  resolve `fire_sound` at spawn. But this observer is registered UNCONDITIONALLY
  by `TurretSectionPlugin` (:338, outside the `if self.render` block that gates
  the render observers which already need `AssetServer`). So every headless rig
  that spawns a turret through the plugin now requires an `AssetServer` it did not
  before - a widened blast radius on a load-bearing observer. It also deviates
  from the established convention in THIS file: `muzzle_effect` stores the
  UNRESOLVED `AssetRef` on a component (`TurretSectionBarrelMuzzleEffect(config.
  muzzle_effect.clone())`, :450) and resolves it later in a render-time observer.
  - Fix: mirror `muzzle_effect` - store `TurretSectionFireSound(Option<AssetRef<
    AudioSource>>)` (the cloned config field, no `AssetServer` at build time), and
    resolve it in `on_turret_fire_play_sfx` (which already runs only in an
    audio-live app and early-returns without the bank). Drop the `AssetServer`
    param from `insert_turret_section`.
  - Response: Done. `TurretSectionFireSound` now carries
    `Option<AssetRef<AudioSource>>` (mirroring `TurretSectionBarrelMuzzleEffect`);
    `insert_turret_section` stores `config.fire_sound.clone()` in the existing
    turret insert with NO `AssetServer` param; `on_turret_fire_play_sfx` gained
    `Res<AssetServer>` and resolves the ref at cue time (idempotent load). The
    always-registered build observer no longer requires `AssetServer`.
- [x] R1.2 (MAJOR) crates/nova_gameplay/src/audio.rs (tests
  `a_turret_with_a_declared_fire_sound_plays_that_handle_not_the_bank`) - the
  playback tests MANUALLY insert a `TurretSectionFireSound` component; nothing
  exercises the real declaration path (`TurretSectionConfig { fire_sound: Some(..)
  }` -> `insert_turret_section` -> resolved handle -> played). If that wiring
  broke, the tests stay green. The task's own step (a) asks for "a turret whose
  CONFIG declares a fire_sound plays THAT handle".
  - Fix: add an App-rig test that spawns a turret from a real `TurretSectionConfig`
    carrying `fire_sound` (through `insert_turret_section` / the plugin, with
    `AssetPlugin`), fires a round, and asserts the played handle equals
    `asset_server.load(<declared path>)` - marrying declaration -> resolution ->
    playback. Keep the fallback test as the delivery guard.
  - Response: Done, as two focused tests covering the two halves of the seam
    (more robust than a full-plugin app that pulls in unrelated turret systems).
    New `turret_section.rs` test
    `insert_turret_section_snapshots_the_configs_fire_sound_onto_the_turret`
    runs the REAL `insert_turret_section` observer and asserts a config-declared
    `fire_sound` becomes the `TurretSectionFireSound(Some(ref))` component (and a
    turret without one gets `None`). The audio test then resolves that component's
    ref and asserts the played handle - declaration -> component -> resolved
    playback. Post-redesign the declaration half needs no `AssetServer`.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/audio.rs (test rig `turret_fire_app`)
  - builds the bank with `SoundBank::load` (old `sounds/<name>.wav` convention)
  while production now uses `SoundBank::load_paths` with `base/sounds/`. Harmless
  today (headless, no disk load; the override test uses a mod path) but fragile.
  - Fix: build the rig bank with `load_paths` and `base/sounds/` paths to mirror
    production, or add a one-line comment that the convention is irrelevant to the
    headless rig.
  - Response: Done. `turret_fire_app` now builds the bank with `load_paths` +
    `base/sounds/<name>.wav`, mirroring production `register_sounds`.
- [x] R1.4 (NIT) crates/nova_assets/src/lib.rs:1075-1087,
  crates/nova_gameplay/src/audio.rs NOVA_SFX_FILES doc - the repointed doc comments
  are correct but verbose. Tighten at the implementer's discretion.
  - Response: Tightened `play_positional_handle`'s doc and corrected the now-stale
    "resolved at spawn" wording (it is snapshotted at spawn, resolved at cue time)
    in the config field doc + design doc. Left the NOVA_SFX_FILES/register_sounds
    docs as-is - they carry the load-bearing "why base/sounds + load_paths"
    rationale.

## Round 2

- VERDICT: APPROVE

All Round 1 findings verified resolved against the new diff (commit c54fa797):

- R1.1 verified: `insert_turret_section` (turret_section.rs:457-461) no longer
  takes `Res<AssetServer>`; `TurretSectionFireSound` now holds
  `Option<AssetRef<AudioSource>>` and is snapshotted in the turret insert;
  `on_turret_fire_play_sfx` gained `Res<AssetServer>` and resolves the ref at cue
  time. Matches the `muzzle_effect` convention; the always-registered build
  observer is `AssetServer`-free again.
- R1.2 verified: new `insert_turret_section_snapshots_the_configs_fire_sound_onto_the_turret`
  exercises the real observer (declaration -> component, both Some and None); the
  audio test resolves the component's ref and asserts the played handle
  (component -> resolved playback). nova_gameplay lib went 520 -> 521.
- R1.3 verified: `turret_fire_app` builds its bank with `load_paths` +
  `base/sounds/`, mirroring production.
- R1.4 verified: stale "resolved at spawn" wording corrected in the config field
  doc, `play_positional_handle` doc, and the design doc.

Re-ran the full check surface in the worktree: nova_gameplay lib 521 passed;
nova_assets mod_refs 19 passed; content_ron_parity + content_lint_gate 4 passed;
`cargo check --workspace --all-targets --features debug` clean (only the
pre-existing examples/19_broadside.rs unused-`mut` warning, not in this diff).

No new findings. The branch delivers the Goal: sections can declare an authorable
`AssetRef<AudioSource>` that ships + references through the `self://`/`dep://base`
pipeline, base sounds live under `assets/base/`, and base playback is unchanged.
