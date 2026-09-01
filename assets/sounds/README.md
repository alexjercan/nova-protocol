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

All non-positional, and all played: every `UiSfx` key has a file (the
`every_ui_sfx_key_has_a_file` test) and every file now has a site that plays
it. A future file with no key is rendered and waiting for its observer - an
unfinished hook, not an error.

| File | Event | Played by |
| --- | --- | --- |
| `menu_select.wav` | a menu button is pressed | `nova_menu::widgets` |
| `menu_back.wav` | a menu or overlay is dismissed | `nova_menu::widgets` (a `back_button`) |
| `menu_focus.wav` | the cursor arrives on an item | `nova_menu::widgets` |
| `ui_toggle.wav` | a setting or overlay changes state | `nova_menu::pause` |
| `ui_tick.wav` | a slider passes a detent | `nova_menu::settings` |
| `objective_new.wav` | an objective is posted | `nova_hud::objective_feedback` |
| `objective_complete.wav` | an objective is completed | `nova_hud::objective_feedback` |
| `objective_fail.wav` | a scenario is lost | `nova_menu::outcome` |
| `comms_line.wav` | a dialogue line opens | `nova_hud::comms_panel` |
| `editor_place.wav` | a part is placed | `nova_editor::cues` |
| `editor_remove.wav` | a part is removed | `nova_editor::cues` |
| `editor_rotate.wav` | the ghost's pose moves | `nova_editor::cues` |
| `editor_deny.wav` | an illegal placement is refused | `nova_editor::cues` |
| `nova_*.wav` (11) | the NOVA OS terminal | `nova_ui` |
