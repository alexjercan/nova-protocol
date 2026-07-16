# Review: Settings menu content (graphics quality, keybinds, audio)

- TASK: 20260711-180511
- BRANCH: menu/settings-content

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed out-of-context (fresh-eyes agent) against the spec, the full branch
diff, the bevy_ui_widgets 0.19 slider source, bevy_audio 0.19 `audio_output.rs`,
the flight/camera/targeting rigs and `juice.rs`. All 11 new tests were run and
pass. Verified sound (no findings): the slider feedback loop cannot cycle
(`slider_self_update` does a plain insert, no re-emit); audio scaling is applied
exactly once (GlobalVolume at sink-spawn for one-shots, manual for the thruster
loop); the three graphics tiers are genuinely distinct and observable; every new
system guards its engine target with `Option<Res<...>>` or self-inits the
resource (no minimal-rig panic); the persistence startup-skip guard
(`is_changed && !is_added`) is logically correct; the keybind table matches the
real rigs row-by-row; pause and main-menu sliders can never coexist; docs
(CHANGELOG, plan, sibling task, wiki) are updated; no leftover files, dead code,
or unused imports.

- [x] R1.1 (MAJOR) crates/nova_menu/src/lib.rs `on_volume_slider_change` +
  `persist_settings_on_change` - dragging the volume slider rewrites the config
  file on EVERY drag frame. `slider_on_drag` emits an intermediate
  `ValueChange { is_final: false }` per pointer-move; each flips `MasterVolume`,
  and `persist_settings_on_change` (reacting to `is_changed` every Update) then
  does a full RON `fs::write` / localStorage write that frame. One drag = dozens
  to hundreds of writes. Fix: keep applying the live value to `MasterVolume`
  every change (audio must track the drag), but persist only once the value
  settles. `is_final` alone is insufficient (a track-click emits only
  `is_final: false`, no final), so debounce the save.
  - Response: Fixed. `persist_settings_on_change` now debounces via a
    `Local<Option<u32>>` idle counter: any change (re)arms it, and the save
    fires once the value holds steady for `SETTINGS_SAVE_DEBOUNCE_FRAMES` (15,
    ~0.25s). A full drag or a track-click now writes exactly once, after it
    settles; `on_volume_slider_change` still applies the live value every frame
    so audio tracks the drag. I verified in the slider source that
    `is_final:true` fires only on drag-end / keyboard (not track-click), which
    is why debounce - not an `is_final` gate - is the correct fix.
- [x] R1.2 (MAJOR) crates/nova_gameplay/src/input/player.rs
  `reference_rows_track_the_flight_rig` - the parity test does not actually pin
  the displayed key strings to the rig. It asserts (a) the rig binds hardcoded
  KeyCodes and (b) `KEYBINDS` has a row with the matching action NAME, but never
  compares a `KEYBINDS` row's `keyboard` display string to the rig's key. A
  remap `W -> T` that also updates the hardcoded KeyCode stays green while the
  on-screen reference still reads "W / Space" - the exact silent desync the
  doc comment claims to prevent. Fix: derive the expected display label from the
  rig's actual bound key and assert the `KEYBINDS` row's `keyboard` contains it.
  - Response: Fixed. The parity test now reads each action's primary key LIVE
    from the rig, maps it to its friendly label with an in-test `display_label`
    (unmapped key -> panic), and asserts the `KEYBINDS` row's `keyboard`
    contains that derived label. A remap now either flips the derived label (row
    no longer contains it -> fail) or hits an unmapped key (panic); the
    hardcoded-KeyCode asserts are gone. Genuinely pins display <-> rig.
- [x] R1.3 (MINOR) crates/nova_menu/src/lib.rs plugin build - confirm
  `UiWidgetsPlugins` (DefaultPlugins) does not also register `slider_self_update`
  (a double `SliderValue` insert per change would be harmless but wasteful).
  - Response: Confirmed, no change needed. Read `SliderPlugin` in bevy_ui_widgets
    0.19 (slider.rs): it registers `slider_on_pointer_down/up/cancel`,
    `slider_on_drag_start/drag/drag_end`, `slider_on_key_input`, the four
    `slider_on_insert*`, and `slider_on_set_value` - NOT `slider_self_update`,
    which is a standalone opt-in `pub fn`. So the menu's single registration is
    the only one; no double insert.
- [x] R1.4 (NIT) crates/nova_menu/src/lib.rs - the startup-skip persistence
  guard has no app-level test (only the pure `save_to`/`load_from` round-trips
  are covered). Consider one that asserts no save fires on launch.
  - Response: Deferred (accepted as a NIT). `persist_settings_on_change` writes
    through the free `save_settings`, which hits the real user config dir /
    localStorage; an app-level test would either write real user files or need a
    save-sink injected (env-var/`XDG_CONFIG_HOME` juggling is racy under the
    parallel test runner and platform-specific). The pure store is unit-tested
    (`save_to`/`load_from` round-trip, missing, corrupt, partial), and the
    `is_added` startup-skip is now additionally covered by the debounce (a
    no-edit launch never arms it). Not worth an injection layer for a settings
    save; left as the documented NIT.

## Round 2

- VERDICT: APPROVE

Re-reviewed the fixes on the same branch; both MAJORs resolved, MINOR confirmed,
NIT accepted as deferred.

- R1.1: `persist_settings_on_change` now debounces through a `Local<Option<u32>>`
  idle counter (SETTINGS_SAVE_DEBOUNCE_FRAMES = 15). A drag's per-frame
  `MasterVolume` changes coalesce to a single write once the value holds steady;
  a track-click (no `is_final`) and a graphics-button press likewise write once.
  The startup add is still skipped via `is_added`, so a no-edit launch never
  writes. Verified one interaction -> one write.
- R1.2: `reference_rows_track_the_flight_rig` derives each row's expected label
  from the rig's LIVE primary key (`display_label`, unmapped -> panic) and
  asserts the `KEYBINDS` row's `keyboard` contains it. A rig remap now flips the
  derived label (row no longer contains it -> fail) or panics on an unmapped
  key. The old hardcoded-KeyCode asserts are gone; the readout is genuinely
  pinned to the rig.
- R1.3: confirmed `SliderPlugin` does not register `slider_self_update`; single
  registration, no double insert.
- R1.4: deferred (NIT) - the pure store is unit-tested; an app-level probe would
  write real user files or need a save-sink injection not worth adding.

Checks after the fixes: gameplay settings 3/3, keybind reference + parity 2/2,
full nova_menu suite 59/59, `cargo fmt --check` clean. APPROVE.
