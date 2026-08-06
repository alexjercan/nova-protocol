# Prototype 07 - the SFX machinery -> `nova_gameplay/src/audio/`

A **merge**, not a drop-in: `crates/nova_gameplay/src/audio/` already exists
and already owns the game-specific half.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/audio/mod.rs` | 137 | `crates/nova_gameplay/src/audio/sfx.rs` |
| `src/audio/registry.rs` | 210 | `crates/nova_gameplay/src/audio/registry.rs` |

347 L, matching NOTES.md.

**Do not overwrite `audio/mod.rs`.** Nova's `audio/mod.rs` is a 40-line
docstring plus the `NovaAudioPlugin` composition root over `combat.rs`,
`cues.rs`, `loops.rs`, `mixing.rs`, `test_support.rs`. BCS's `audio/mod.rs`
holds `PlaySfx` / `SfxMasterVolume` / `SfxCommandsExt` / `SfxPlugin` - it is a
peer module, so land it as `audio/sfx.rs` beside the others.

## Exports that must survive

`SfxPlugin`, `PlaySfx`, `SfxCommandsExt`, `SfxMasterVolume` (from `sfx.rs`),
`SoundBank` (from `registry.rs`).

`nova_gameplay/src/lib.rs:77` re-exports `PlaySfx`, `SfxCommandsExt`,
`SfxPlugin`, `SoundBank` by name. `SoundBank` is generic and `nova_assets`
inserts `SoundBank<UiSfx>` - so `nova_assets` reaches it through
`nova_gameplay::prelude`. Those four must keep resolving.

## The docstring is the interesting part

`nova_gameplay/src/audio/mod.rs:3-7` currently says:

> The generic playback machinery lives in `bevy_common_systems`: [`SfxPlugin`]
> spawns a self-despawning audio entity for every [`PlaySfx`], and [`SoundBank`]
> is a keyed registry of loaded handles. This module owns only the
> *game-specific* part - the mapping from Nova gameplay events to sounds - so
> the reusable half stays promotable and this half stays Nova's.

That sentence is the whole reason this task exists. Rewrite it: nova owns both
halves now, `sfx.rs` is the playback machinery and `registry.rs` the keyed
bank, and the rest of the module is the Nova-specific cue mapping. Keep the
"promotable" idea if you want - it is exactly the owner's plan for a future
`bevy-common-systems` rebuilt out of nova - but stop claiming another crate
holds the code.

Everything from `mod.rs:9` down (the four one-shot cues, the distance
attenuation policy, the thruster hum) stays verbatim.

## Module wiring

In `audio/mod.rs`:

```rust
pub mod registry;
pub mod sfx;
```

`NovaAudioPlugin::build` should add `sfx::SfxPlugin` itself, so the audio
plugin owns its whole stack. Today `SfxPlugin` is **not** in
`plugin.rs:81-106` - check where it is registered before moving it
(`grep -rn 'SfxPlugin' crates/`), and keep the count at exactly one.

Extend `audio`'s export line in the crate prelude. The current entry
(`lib.rs:88-91`) lists nova's own names explicitly
(`NovaAudioPlugin`, `SfxListenerMarker`, `UiSfx`, the volume constants) - add
the five copied names to that list, and delete them from the BCS block at
`lib.rs:77`.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/hud/comms_panel.rs` | 21 | `SfxCommandsExt`, `SoundBank` |
| `nova_gameplay/src/hud/nova_os/input.rs` | 9 | `SoundBank` (with `GameObjectives`, prototype 08) |
| `nova_gameplay/src/hud/nova_os/shell.rs` | 2 | `SoundBank` |
| `nova_gameplay/src/hud/nova_os/sound.rs` | 2 | `SfxCommandsExt`, `SoundBank` |
| `nova_gameplay/src/hud/nova_os/tests/mod.rs` | 29 | `PlaySfx`, `SoundBank` (+ `GameObjectives`, `Objective`) |
| `nova_gameplay/src/audio/*` | - | `mod.rs` and children reach these through `crate::prelude::*` today; they will resolve through the same glob after wiring |

Nothing outside `nova_gameplay` names them directly - `nova_assets` goes
through the prelude.

## Compile hazards

- `audio/mod.rs` (BCS) needs `bevy::audio::Volume`; `registry.rs` is pure bevy.
  No new deps. `nova_gameplay` already enables bevy's `wav` feature
  (`Cargo.toml:17`) for the placeholder SFX.
- 1 `bevy_common_systems` string in `audio/mod.rs`, 2 in `registry.rs` - all
  doctest `use` lines.
- The BCS `audio/mod.rs` header has a runnable doctest (`commands.play_sfx`,
  `commands.trigger(PlaySfx::new(..))`) that needs its `use` line repointed at
  `nova_gameplay::prelude::*`.
- Name clash check: nova's `audio/mixing.rs` owns `SfxListenerMarker` and
  volume policy; BCS's `SfxMasterVolume` is a separate resource. Confirm
  `mixing.rs` is not already defining something named `SfxMasterVolume` before
  copying - if it is, that is a real design question, not a rename, and it
  should be recorded rather than silently resolved.
- `#![warn(missing_docs)]`: BCS's `pub mod prelude` in `audio/mod.rs` has no
  doc comment.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib audio
nix develop --command cargo test -p nova_gameplay --lib hud::nova_os
nix develop --command cargo check -p nova_assets --all-targets
nix develop --command cargo fmt --check
```

If `SfxPlugin` registration moves, **run** an example under Xvfb `:99`. Audio
regressions are silent in every automated check here - a duplicated `SfxPlugin`
would double every sound and nothing would fail. Verify the registration count
by reading, not by testing.

## Done when

- `nova_gameplay/src/audio/` holds `sfx.rs` + `registry.rs` beside the existing
  five modules.
- `audio/mod.rs`'s docstring no longer claims BCS owns the playback half.
- `SfxPlugin` is added exactly once.
- `nova_assets` still resolves `SoundBank<UiSfx>` through
  `nova_gameplay::prelude`.
