# Turn wfc_arena into a configurable match loop

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.11.0,example,ui,editor,nova-os,input

## Goal

Turn `wfc_arena` into the first small configurator reference for future editor
work. Add reusable text-entry and select-list widgets, a pre-match lobby, a
pause/return flow, an authoritative match result, a dynamic result scoreboard,
and selected-section rebinding in the NOVA OS ship viewer.

Keep arena rules example-specific. Put reusable controls in `nova_ui`. Do not
build a general match framework or an embedded hull preview in this task.
`wfc_ships --seed <seed>` remains the hull-preview path.

## Accepted design

### Lobby

- Normal hand-runs open the lobby. Existing CLI arguments prepopulate it.
- `NOVA_AUTOPILOT=1` bypasses the lobby and starts the configured match.
- Two fixed sides remain Amber and Onyx, with at most four ships per side and at
  most one local Player slot in total.
- Style is selected once per side, not per ship.
- Each ship row has an AI/Player selector, one always-populated numeric seed text
  field, a button that replaces it with another combat-viable seed, and a remove
  button. The seed is exact and editable; there is no Draft/Pinned mode.
- Initial values resolve from the existing CLI/default roster. `--style`
  initializes both sides; explicit per-ship CLI styles apply in command order,
  so the last explicit style for each side wins. Exact CLI pins remain exact.
  Automatically filled and rerolled values pass the armament floor.
- Side controls add ships. Global controls start the match or quit.
- No global draft-seed field, global default-style field, team-name editor,
  color editor, AI difficulty, composition editor, or live 3D preview.
- The lobby demonstrates reusable text entry and validation, select lists,
  dynamic rows, buttons, keyboard focus, mouse activation, and disabled/invalid
  states.

### Shared text field

- Public API: `TextFieldSpec::new(value)`, `text_field(spec)`,
  `TextFieldValue`, `TextFieldFocused`, `TextFieldError`, and
  `TextFieldSubmitted`.
- Values update on each edit. Enter and outside clicks submit. Escape restores
  the focus-entry value and exits.
- Support character input, Backspace, Delete, arrows, Home, and End. Default to
  256 characters. Validation belongs to the caller.
- No clipboard, selection range, undo stack, or IME composition in this slice.
- Style selection composes the existing visible `ListRow` widgets instead of
  adding an overlapping select-control API.

### Match and pause

- Remove the top in-match score text and the in-match reroll/style controls.
- Keep AI-spectator camera controls and normal Player controls.
- Escape opens Resume, Restart Match, Return to Lobby, and Quit.
- Restart uses the same resolved ships and seeds. Return keeps lobby values.
- NOVA OS escape precedence stays app -> terminal -> flight pause.

### Result

- A side wins when the opposing side has no ship with a live flight computer
  after all configured combatants have spawned. Both sides reaching zero in one
  destruction batch is a draw.
- A ship outside the 20 km arena sphere shows a 30-second disqualification
  countdown. Re-entry resets it; expiry destroys its live flight computers.
- Ammunition fired, damage, and non-zero fighter thrust are global activity.
  After 180 seconds without activity, remaining team structure percentage
  decides a stalemate advantage; exact equality is a draw.
- Freeze combat and bodies at declaration, then show the result after a short
  finish delay.
- Show result, duration, and per-team starting ships, survivors, remaining
  structure, damage dealt, and shots by observed ammunition type.
- Show each ship's slot, seed, style, survival, and remaining structure.
- Result actions: Restart Match, Return to Lobby, Quit.
- No per-ship damage or kill attribution in this slice.
- Ammunition rows are dynamic. Bullet labels come from projectile `DamageType`;
  torpedo labels come from authored `TorpedoType.name`. Use a sorted map and
  total observed shots for probe predicates. Adding an ammunition type must not
  require arena scoreboard changes.

### Rebinding

- The selected NOVA OS ship-viewer section shows bindings only when it has a
  runtime binding component.
- `B Rebind` captures one keyboard key or mouse button. Escape cancels.
- Replace only the selected section's complete binding list.
- Reject a binding used by another section on the same ship and identify the
  conflict.
- Emit a generic binding-change message. The arena stores overrides by roster
  slot and stable section id, preserving them through restart and return.
- No disk persistence, same-type group edit, gamepad capture, or general control
  remapping.

## Delivery slices

1. Reusable text field/select list and the arena lobby.
2. Pause, restart, and return-to-lobby flow.
3. End detection and dynamic result scoreboard.
4. Ship-viewer selected-section rebinding and arena override retention.
5. Rendered player-path coverage, docs, review, and retro.

Stop for owner playtest after the lobby slice before hardening later UI around
it.

## Done when

- A hand-run configures both sides with reusable text fields, lists, and buttons,
  then starts a match.
- Operational defeat, arena disqualification, and inactivity stalemate rules
  prevent a match from blocking the result screen indefinitely.
- Pause and result screens restart or return without stale scenario entities.
- Result rows adapt to the projectile ammunition types observed in the match.
- A Player ship can rebind one selected bindable section through NOVA OS, with
  conflict and restart-retention coverage.
- Existing CLI and driven probe paths remain deterministic.
