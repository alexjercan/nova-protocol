# Notes - two sections, one keybind

## The change

`binding_conflict` (`nova_editor/src/keybind.rs`) lost its section-vs-section
arm and with it both unused arguments. It now takes the binding alone and
answers one question: does the always-on flight rig already drive this source?
Nothing else about the rebind path moved.

## Why the deleted arm was never needed

Checked the history before deleting, as the brief asked.

- The arm arrived in `f9a52831` ("L6 - nova_editor's five defects", finding
  F32) from task `20260806-121625`.
- That task's finding table says the defect was "click-to-rebind accepts any
  key with NO conflict check. Authored content with that mapping is rejected by
  `scenario_input_overlaps`". Its plan line asks only to "reuse the lint's rule
  rather than restating it".
- `scenario_input_overlaps` compares player bindings against
  `flight_rig_reserved_sources` and nothing else. The sibling-section rule was
  never in the lint, so F32 over-applied its own brief. No runtime reason is
  recorded anywhere, and none exists (below).

## Why sharing is safe

- Every section binding rig sets `ActionSettings { consume_input: false }`
  (`nova_ship/src/input/player/weapons.rs`, all three kinds), on its OWN entity.
  Two sections on one source both fire. That is the same mechanism that makes a
  flight-rig overlap a BUG - the key drives flight as well - and a
  section-section overlap a CHOICE.
- The shipped content already does it: `broadside`, `shakedown_run` and
  `final_tally` bind `turret_port` and `turret_starboard` both to `Mouse(Left)`
  and `Gamepad(RightTrigger2)`, and they pass `content lint`. The editor was
  the only surface refusing what the game already ships.

## Nothing else assumed one section per source

- Chips: `sync_section_keybind_labels` spawns one label per bindable SECTION
  and `position_section_keybind_labels` reads each section's own binding. Two
  chips reading `K` is the correct picture.
- `PlayerSpaceshipConfig::inputs` is `HashMap<Entity, Vec<Binding>>` and the
  scenario `input_mapping` is keyed by section id, so a source can appear twice
  on the way out.
- `nova_hud::key_glyphs` is a label -> keycap lookup with no section in it, and
  the keybind dock renders FLIGHT verbs only.
- `placement::capture_binding` excludes the editor camera keys, never siblings.

## Proof

- `cargo test --lib -p nova_editor`: 66 pass.
  `rebind_refuses_a_key_another_section_already_holds` is inverted into
  `rebind_lets_two_sections_share_one_key` (both sections keep the key, the
  rebind is consumed, both reach `PlayerSpaceshipConfig::inputs`).
  `rebind_refuses_a_key_the_flight_rig_drives` is unchanged and still passes.
- LIVE, through the real editor (a throwaway autopilot example on the real
  `editor_app`, deleted after the run; Xvfb, exit 0):
  - two PDC turrets placed through the gallery, each rebound onto `K` by
    clicking the section in Select mode - the SECOND one now takes it;
  - the hand-off carried both: `input_mapping: {"1761v1": [KeyK], "1540v3":
    [KeyK]}`;
  - after Play, holding `K` read `turret triggers [true, true]` - BOTH turrets
    fire;
  - rebinding onto `G` was still refused, with the unchanged message
    `already driven by the flight rig's autopilot goto - pick another key`.
