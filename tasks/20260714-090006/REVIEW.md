# Review: UI/feedback SFX pass (20260714-090006)

Branch: `feat/ui-feedback-sfx`

## Verdict: APPROVE

Self-review found no correctness bugs; a few design decisions are recorded below.

## Scope reviewed

Four new `NovaSfx` cues + placeholder WAVs (generator entries) + wiring:

- **MenuSelect** - global `On<Activate>` observer in nova_menu, clicks for any
  `MenuButton` (every menu/pause/mods button carries it via `button()`).
- **UiToggle** - ESC pause-overlay open/close in `toggle_pause`.
- **DryFire** - `play_dry_fire_cue` in audio.rs: edge-latched per turret, plays
  on the rising edge of (trigger held + weapons hot + magazine empty), player
  only (`q_ship` filtered to `PlayerSpaceshipMarker`).
- **RadarRetarget** - new `RadarRetargeted` message fired in
  `update_radar_search` when an already-acquired gesture changes to a new
  candidate; played by `play_lock_cues`, suppressed on the acquire frame.

## Findings / decisions

### D1 (design) - settings/mods "expand/collapse" rides the menu-button click

The task lists "Settings panel expand/collapse" as its own cue. The Settings
(and Mods) buttons carry `MenuButton`, so pressing them already fires MenuSelect
- that press IS the toggle gesture. A separate panel-visibility cue would
double-sound the same action, so it was deliberately not added. Every listed
silent interaction now makes a sound.

### D2 (design) - pause close path differs by gesture

ESC open/close plays UiToggle; closing via the Resume/Exit buttons plays
MenuSelect (they are buttons). Both give feedback; the slight asymmetry is
acceptable and simpler than special-casing.

### R1 (MINOR, accepted) - dry-fire latch not pruned

`play_dry_fire_cue` keeps a `Local<HashMap<Entity, bool>>` of per-turret edge
state. Entries for despawned turrets are never removed (the query only iterates
live turrets, so stale entries are dead weight, not a correctness issue). Turret
counts are tiny; growth is negligible. Left unpruned for simplicity.

### R2 (MINOR, accepted) - README table

Same as task 20260714-090002: `assets/sounds/README.md`'s "Required files" table
still lists only the 5 original core cues. `NOVA_SFX_FILES` + the generator dict
are the source of truth; a full table refresh is a separate docs chore.

## Correctness checks done

- `RadarRetargeted` fires only AFTER acquire (acquire takes the `!acquired`
  branch; retarget is the `else if changed`), so acquire is never double-cued.
  The acquire-frame suppression in `play_lock_cues` prevents a LockOn+tick chord.
- `RadarRetargeted` is drained every frame (bank and no-bank paths) and
  auto-clears via `add_message`, so it cannot accumulate.
- Two headless targeting test rigs run `update_radar_search`; both were given
  the new message resource so they do not panic on the new `MessageWriter`.
- Dry-fire is gated to player + hot + empty + held, edge-latched (no per-frame
  buzz), and pause-safe (frozen input yields no fresh edge).

## Tests (all green; workspace `cargo check` + `cargo fmt --check` clean)

- `dry_fire_clicks_on_the_empty_pull_edge_then_stays_quiet_while_held`
- `dry_fire_is_gated_to_the_player_hot_and_empty`
- `a_retarget_within_a_held_gesture_ticks_but_the_acquire_does_not`
- `a_menu_button_activation_clicks_and_a_bare_activation_does_not`
- `the_escape_pause_toggle_blips_on_both_directions`
- `every_nova_sfx_key_has_a_file` widened to all 16 keys.

Full suite deferred to CI per project convention.
