# UI sound effects (engine chrome)

The game's INTERFACE cues - menu clicks and objective chimes. Like
`assets/icons/`, these are engine assets loaded directly from the asset root:
they are NOT part of any mod, never appear in a bundle's `resources`, and
cannot be referenced by content (`self://`/`dep://` do not reach here). The
world/gameplay sounds live with the base mod in `assets/base/sounds/` - see the
README there and spike `tasks/20260717-101524/SPIKE.md` for the ownership
split.

The files are **tiny generated placeholders** produced by
`scripts/gen-placeholder-sounds.py`, not final sound design. They load through
`SoundBank::load`'s `sounds/<name>.wav` convention into the `UiSfx` bank
(`register_sounds` in `crates/nova_assets/src/lib.rs`); the keys are
`UiSfx` in `crates/nova_gameplay/src/audio.rs`, guarded by the
`every_ui_sfx_key_has_a_file` test. To replace one, drop a real sound at the
same path and filename; to regenerate the placeholders run
`python3 scripts/gen-placeholder-sounds.py` from the repo root.

## Required files (non-positional)

| File | Event | Character / length |
| --- | --- | --- |
| `objective_new.wav` | A new objective is posted to the panel | short neutral blip, ~0.12 s |
| `objective_complete.wav` | An objective is completed | rising fifth (success), ~0.22 s |
| `menu_select.wav` | A menu button is pressed (New Game / Sandbox / Settings / Exit, pause, mods) | crisp rising click, ~0.06 s |
| `ui_toggle.wav` | The pause overlay toggles open/close (ESC) | soft two-state blip, ~0.05 s |
