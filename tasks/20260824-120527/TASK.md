# Tabbed settings menu with keyboard and gamepad rebinding

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.12.0,ui,input

v0.12.0. The settings half split out of `20260714-001140` (which keeps
gamepad menu navigation + the mobile pad, backlog). Depends on the bindings
registry from `20260820-174148` phase 1. Research:
`tasks/20260815-231945/INPUT-AND-PROCESS.md` sections 2-3 and 5.

## Goal

Turn Settings into a real settings menu: a tabbed layout (Audio / Graphics /
Controls / Interface), and rebinding for keyboard AND gamepad, reading and
writing the bindings registry instead of the hand-authored display mirror.

## Reuse - all three flows are tested code already in tree

- Tabs: the mods screen pattern verbatim - `ModsActiveTab` resource
  (nova_menu/src/mods.rs:51-52), `on_mods_tab` (:226-249), visuals
  `segmented_container` / `segmented_option`
  (nova_ui/src/widget/segmented.rs:34-50).
- Rebind capture: `apply_section_rebind` (nova_editor/src/keybind.rs:197-279)
  - armed-target resource, capture next key or mouse press, Escape cancels,
  waits out the arming click, refuses conflicts, stays armed on refusal.
  Generalise from section entities to action names. Second copy at
  nova_os_ui/src/ship/rebind.rs. NEITHER captures gamepad buttons - the
  gamepad capture branch is the genuinely new piece.
- Persistence: `PersistedSettings` (nova_menu/src/settings_store.rs:16-36),
  RON key "settings", debounced save, exit flush. Add a serde-defaulted
  `bindings: map<action_name, bindings>` field; copy the partial-file test
  pattern (settings_store.rs:213-230). Apply loaded bindings by patching the
  rig's `Binding` child entities on rig spawn (read pattern:
  hints.rs:227-242).

## What this deletes

- The FLIGHT and TARGETING rows of the hand-authored mirror
  (nova_ship/src/input/reference.rs, TODO(20260710-231927) at :10) and their
  parity test (hints.rs:352-449): the Controls tab renders from the registry.
- `flight_rig_reserved_sources()` (hints.rs:164-195), the SECOND
  hand-maintained mirror: conflict checks must compute from the live
  registry, or they go stale on the first remap. `nova_hud/src/key_glyphs.rs`
  needs the same live source.

## Fixed rows

The raw system chords stay non-rebindable this release and are LISTED as
fixed: pause Esc/Start, HUD backquote/Select, NOVA OS Tab/RightThumb, comms
V/B, scenario advance Enter/DPadDown. (Inventory:
INPUT-AND-PROCESS.md section 5.)

## Done when

- Settings shows tabs; Controls lists every registry action with its
  keyboard and gamepad bindings, live.
- Rebind a flight key and a pad button; conflicts refused with a reason;
  bindings survive a restart; the rig is built from them.
- The mirror rows and parity test are deleted; reserved-source conflict
  checks are registry-derived.
- Works from both entry points (main menu overlay and pause overlay).
