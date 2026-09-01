# Interface sound effects (engine chrome)

The game's INTERFACE cues - menus, the editor, objectives, comms. Like
`assets/icons/`, these are engine assets loaded directly from the asset root:
they are NOT part of any mod, never appear in a bundle's `resources`, and
cannot be referenced by content (`self://` / `dep://` do not reach here). The
world, cockpit and avionics sounds live with the base mod in
`assets/base/sounds/`.

## What lives here, and why

The split is not UI against SFX. It is **where the variation lives**:

- What the ENGINE plays uniformly for everyone, and what still has an event to
  fire on with zero mods loaded, is chrome and belongs here.
- What CONTENT authors per thing - because two of that thing could reasonably
  differ - belongs in `assets/base/sounds/` behind an `AssetRef`.

That is why the cockpit is not here: locking is a CAPABILITY of a controller
section, so two controllers are allowed to sound different. See
`tasks/20260717-101524/SPIKE.md` for the original ownership split and
`tasks/20260824-125955/` for the voices.

## The voice

Two files make this directory. The eleven `nova_*.wav` are the STANDARD -
baked from the NOVA OS terminal recipes by `scripts/gen-nova-os-sfx.py`
(stdlib only, no numpy) - and everything else is written to join them by
`scripts/gen-ui-sfx.py`, which shares their primitives: a square or triangle
oscillator with an exponential pitch slide, a short noise blip through one
resonant band, and that family's attack/decay envelope.

    nix develop --command python3 scripts/gen-ui-sfx.py

Every cue seeds its own generator from a hash of its NAME, so a rerun is
byte-identical and adding a cue rewrites no other cue's bytes. Files are peak
normalized to -3 dBFS: balance between cues is NOT set here, the `UiSfx`
volumes do that.

To replace one by hand, drop a real sound at the same path and filename. They
load through `SoundBank::load`'s `sounds/<name>.wav` convention into the
`UiSfx` bank (`register_sounds` in `crates/nova_assets/src/lib.rs`); the keys
are `UiSfx` in `crates/nova_gameplay/src/audio.rs`, guarded by the
`every_ui_sfx_key_has_a_file` test.

## Files

All non-positional. A file with no `UiSfx` key yet is rendered and waiting for
the observer that plays it - a file nothing references is an unfinished hook,
not an error.

| File | Event | Keyed |
| --- | --- | --- |
| `menu_select.wav` | a menu button is pressed | yes |
| `menu_back.wav` | a menu or overlay is dismissed | no |
| `menu_focus.wav` | the cursor moves between items | no |
| `ui_toggle.wav` | a setting or overlay changes state | yes |
| `ui_tick.wav` | a slider or stepper passes a detent | no |
| `objective_new.wav` | an objective is posted | yes |
| `objective_complete.wav` | an objective is completed | yes |
| `objective_fail.wav` | an objective is failed | no |
| `comms_line.wav` | a dialogue line opens | no |
| `editor_place.wav` | a part is placed | no |
| `editor_remove.wav` | a part is removed | no |
| `editor_rotate.wav` | a part is rotated one detent | no |
| `editor_deny.wav` | an illegal placement is refused | no |
| `nova_*.wav` (11) | the NOVA OS terminal | yes |
